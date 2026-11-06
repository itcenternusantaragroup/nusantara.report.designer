//! Reads Stimulsoft .mrt files (JSON and XML flavours) into the designer model.
//!
//! Stimulsoft stores sizes in the report unit (centimetres, inches, millimetres,
//! hundredths of an inch) and stores a landscape page with width > height.
//! Everything is converted to millimetres here, and each page is kept upright
//! (width < height) with an orientation flag, which is what the designer expects.
//!
//! XML files are converted into the same `serde_json::Value` shape as JSON
//! files, so both formats go through one set of readers below.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use uuid::Uuid;

use super::dictionary::{Column, DataSource, Dictionary, Variable};
use super::mrt::{MrtHandler, XmlElement};
use super::report::*;

impl MrtHandler {
    /// Parse a Stimulsoft JSON .mrt. `base_dir` is the folder the file came from; it is used to
    /// find data files that the report points to with a path from another machine.
    pub fn from_stimulsoft_json_str(content: &str, base_dir: Option<&Path>) -> Result<Report, String> {
        let v: Value = serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {e}"))?;
        let mut report = Report::blank();
        read_meta(&v, &mut report);
        let unit = unit_factor(v.get("ReportUnit").and_then(|x| x.as_str()));

        if let Some(dict) = v.get("Dictionary") {
            read_dictionary_json(dict, &mut report, base_dir);
        }

        let pages = v.get("Pages").cloned().unwrap_or(Value::Null);
        build_pages(&mut report, &pages, unit);
        Ok(report)
    }

    /// Parse a Stimulsoft XML .mrt. See `from_stimulsoft_json_str` for `base_dir`.
    pub fn from_stimulsoft_xml_str(content: &str, base_dir: Option<&Path>) -> Result<Report, String> {
        let root = XmlElement::parse(content).map_err(|e| format!("XML parse error: {e}"))?;
        if root.tag != "StiSerializer" {
            return Err("Root XML element is not <StiSerializer>".to_string());
        }

        let mut report = Report::blank();
        let unit = unit_factor(root.find_child("ReportUnit").map(|n| n.text.trim()));

        if let Some(dict) = root.find_child("Dictionary") {
            read_dictionary_xml(dict, &mut report, base_dir);
        }

        // Pages become a list of page objects in the same shape the JSON reader uses.
        let pages: Vec<Value> = root
            .find_child("Pages")
            .map(|p| {
                p.children
                    .iter()
                    .map(|page| {
                        let mut obj = xml_object(page);
                        obj.entry("Name").or_insert_with(|| Value::String(page.tag.clone()));
                        Value::Object(obj)
                    })
                    .collect()
            })
            .unwrap_or_default();
        build_pages(&mut report, &Value::Array(pages), unit);
        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// Units and small value helpers
// ---------------------------------------------------------------------------

/// Millimetres per one report unit. Stimulsoft defaults to centimetres.
fn unit_factor(unit: Option<&str>) -> f32 {
    let u = unit.unwrap_or("").trim().to_ascii_lowercase();
    if u == "inches" || u == "inch" {
        25.4
    } else if u == "millimeters" || u == "millimetres" || u == "millimeter" {
        1.0
    } else if u.contains("hundredth") {
        0.254
    } else {
        10.0
    }
}

/// A yes/no property stored as a JSON bool or as text ("True" in XML).
fn flag_of(v: &Value, key: &str) -> bool {
    match v.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s.trim().eq_ignore_ascii_case("true"),
        _ => false,
    }
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

/// Reads a number that may be stored as a JSON number or as text (XML).
fn num_of(v: &Value, key: &str) -> Option<f32> {
    match v.get(key)? {
        Value::Number(n) => n.as_f64().map(|x| x as f32),
        Value::String(s) => s.trim().parse::<f32>().ok(),
        _ => None,
    }
}

/// Parses "x,y,w,h" (or a JSON object form) into mm.
fn parse_rect(v: Option<&Value>, unit: f32) -> RectMm {
    let fallback = RectMm::new(10.0, 10.0, 40.0, 10.0);
    let Some(s) = v.and_then(|x| x.as_str()) else { return fallback };
    let parts: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse::<f32>().ok()).collect();
    if parts.len() == 4 {
        RectMm::new(parts[0] * unit, parts[1] * unit, parts[2] * unit, parts[3] * unit)
    } else {
        fallback
    }
}

// ---------------------------------------------------------------------------
// Colours, fonts, borders
// ---------------------------------------------------------------------------

/// Common .NET / CSS colour names used in Stimulsoft files.
pub fn named_color(name: &str) -> Option<[u8; 4]> {
    let rgb: [u8; 3] = match name.to_ascii_lowercase().as_str() {
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        "whitesmoke" => [245, 245, 245],
        "silver" => [192, 192, 192],
        "gray" | "grey" => [128, 128, 128],
        "dimgray" | "dimgrey" => [105, 105, 105],
        "darkgray" | "darkgrey" => [169, 169, 169],
        "lightgray" | "lightgrey" | "gainsboro" => [211, 211, 211],
        "red" => [255, 0, 0],
        "maroon" => [128, 0, 0],
        "green" => [0, 128, 0],
        "lime" => [0, 255, 0],
        "darkseagreen" => [143, 188, 139],
        "lightgreen" => [144, 238, 144],
        "blue" => [0, 0, 255],
        "navy" => [0, 0, 128],
        "lightblue" => [173, 216, 230],
        "skyblue" => [135, 206, 235],
        "steelblue" => [70, 130, 180],
        "royalblue" => [65, 105, 225],
        "teal" => [0, 128, 128],
        "aqua" => [0, 255, 255],
        "yellow" => [255, 255, 0],
        "lightyellow" => [255, 255, 224],
        "gold" => [255, 215, 0],
        "khaki" => [240, 230, 140],
        "orange" => [255, 165, 0],
        "navajowhite" => [255, 222, 173],
        "wheat" => [245, 222, 179],
        "tan" => [210, 180, 140],
        "beige" => [245, 245, 220],
        "honeydew" => [240, 255, 240],
        "ivory" => [255, 255, 240],
        "azure" => [240, 255, 255],
        "snow" => [255, 250, 250],
        "seashell" => [255, 245, 238],
        "linen" => [250, 240, 230],
        "lavender" => [230, 230, 250],
        "pink" => [255, 192, 203],
        "brown" => [165, 42, 42],
        "olive" => [128, 128, 0],
        "purple" => [128, 0, 128],
        "fuchsia" => [255, 0, 255],
        _ => return None,
    };
    Some([rgb[0], rgb[1], rgb[2], 255])
}

/// Parses a Stimulsoft colour: `[A:R:G:B]`, `[R:G:B]`, `R,G,B`, or a colour name.
fn parse_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let numbers = |inner: &str, sep: char| -> Vec<u8> {
        inner.split(sep).filter_map(|p| p.trim().parse::<u8>().ok()).collect()
    };
    if s.starts_with('[') && s.ends_with(']') {
        let n = numbers(&s[1..s.len() - 1], ':');
        return match n.len() {
            4 => Some([n[1], n[2], n[3], n[0]]), // [A:R:G:B]
            3 => Some([n[0], n[1], n[2], 255]),
            _ => None,
        };
    }
    if s.contains(',') {
        let n = numbers(s, ',');
        return match n.len() {
            3 => Some([n[0], n[1], n[2], 255]),
            4 => Some([n[0], n[1], n[2], n[3]]),
            _ => None,
        };
    }
    named_color(s)
}

/// Fill colour for a `Brush` value. `None` means "not understood, keep default".
fn parse_brush(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    if lower == "transparent" || lower == "emptybrush" || lower == "empty" || s.is_empty() {
        return Some([0, 0, 0, 0]);
    }
    parse_color(s.strip_prefix("solid:").unwrap_or(s))
}

/// Parses `Font`: "Arial;11;Bold, Italic;" (JSON) or "Arial,11,Bold" (XML).
fn apply_font(comp: &mut Component, s: &str) {
    let sep = if s.contains(';') { ';' } else { ',' };
    let parts: Vec<&str> = s.split(sep).collect();
    let name = parts.first().map(|p| p.trim()).unwrap_or("");
    if !name.is_empty() {
        comp.font_family = name.to_string();
    }
    if let Some(sz) = parts.get(1).and_then(|p| p.trim().parse::<f32>().ok()) {
        comp.font_size = sz;
    }
    let flags = parts.get(2..).map(|f| f.join(",").to_ascii_lowercase()).unwrap_or_default();
    comp.bold = flags.contains("bold");
    comp.italic = flags.contains("italic");
    comp.underline = flags.contains("underline");
}

/// Parses `Border`: "All;Black;1;Solid;False;4;Black" or "Top, Left, Bottom;[193:152:89];1;...".
fn apply_border(comp: &mut Component, s: &str) {
    let parts: Vec<&str> = s.split(';').collect();
    let sides_str = parts.first().map(|p| p.trim().to_ascii_lowercase()).unwrap_or_default();

    let sides = if sides_str == "all" {
        BorderSides::all()
    } else if sides_str.is_empty() || sides_str == "none" {
        BorderSides::none()
    } else {
        let mut b = BorderSides::none();
        for side in sides_str.split(',').map(|x| x.trim()) {
            match side {
                "left" => b.left = true,
                "top" => b.top = true,
                "right" => b.right = true,
                "bottom" => b.bottom = true,
                _ => {}
            }
        }
        b
    };

    if let Some(c) = parts.get(1).and_then(|c| parse_color(c)) {
        comp.border_color = c;
    }
    // Stimulsoft border size is in points; 1 pt is about 0.35 mm.
    let size = parts.get(2).and_then(|x| x.trim().parse::<f32>().ok()).unwrap_or(1.0);
    comp.border_width = (size * 0.3528).max(0.1);
    comp.border_sides = sides;
}

// ---------------------------------------------------------------------------
// Dictionary (data sources and variables)
// ---------------------------------------------------------------------------

/// Resolves a data file path stored in the report. The stored path is often from another
/// machine, so fall back to the file name next to the report, or in a sibling `json` folder.
fn resolve_data_path(raw: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if Path::new(raw).is_file() {
        return Some(PathBuf::from(raw));
    }
    let file_name = raw.rsplit(['\\', '/']).next()?;
    let base = base_dir?;
    [
        base.join(file_name),
        base.join("..").join("json").join(file_name),
        base.join("json").join(file_name),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// Loads sample rows from the referenced data files and merges them into the dictionary.
/// A loaded source replaces an embedded source with the same name.
fn merge_data_paths(report: &mut Report, paths: &[String], base_dir: Option<&Path>) {
    for raw in paths {
        let Some(path) = resolve_data_path(raw, base_dir) else { continue };
        if let Ok(sources) = Dictionary::load_from_json_file(&path) {
            for ds in sources {
                report.dictionary.data_sources.retain(|d| d.name != ds.name);
                report.dictionary.data_sources.push(ds);
            }
        }
    }
}

fn read_dictionary_json(dict: &Value, report: &mut Report, base_dir: Option<&Path>) {
    let mut paths = Vec::new();
    if let Some(dbs) = dict.get("Databases").and_then(|x| x.as_object()) {
        for db in dbs.values() {
            if let Some(p) = db.get("PathData").and_then(|x| x.as_str()) {
                paths.push(p.to_string());
            }
        }
    }

    if let Some(ds_map) = dict.get("DataSources").and_then(|x| x.as_object()) {
        for ds_val in ds_map.values() {
            let name = str_of(ds_val, "Name");
            let name = if name.is_empty() { "Data".to_string() } else { name.to_string() };
            let alias = str_of(ds_val, "Alias");
            let alias = if alias.is_empty() { name.clone() } else { alias.to_string() };

            let mut columns = Vec::new();
            if let Some(cols) = ds_val.get("Columns").and_then(|x| x.as_object()) {
                for col in cols.values() {
                    let c_name = str_of(col, "Name");
                    if c_name.is_empty() {
                        continue;
                    }
                    let c_type = str_of(col, "Type");
                    columns.push(Column {
                        name: c_name.to_string(),
                        data_type: if c_type.is_empty() { "System.String".into() } else { c_type.to_string() },
                    });
                }
            }

            let rows: Vec<std::collections::HashMap<String, String>> = ds_val
                .get("Rows")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_object())
                        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                        .collect()
                })
                .unwrap_or_default();

            report.dictionary.data_sources.push(DataSource { name, alias, columns, rows });
        }
    }

    if let Some(vars) = dict.get("Variables").and_then(|x| x.as_object()) {
        for var in vars.values() {
            let name = str_of(var, "Name");
            let name = if name.is_empty() { "Var".to_string() } else { name.to_string() };
            let alias = str_of(var, "Alias");
            let alias = if alias.is_empty() { name.clone() } else { alias.to_string() };
            report.dictionary.variables.push(Variable {
                name,
                alias,
                value: str_of(var, "Value").to_string(),
                var_type: str_of(var, "Type").to_string(),
                running: var.get("Running").and_then(|x| serde_json::from_value(x.clone()).ok()),
            });
        }
    }

    merge_data_paths(report, &paths, base_dir);
}

fn read_dictionary_xml(dict: &XmlElement, report: &mut Report, base_dir: Option<&Path>) {
    let mut paths = Vec::new();
    if let Some(dbs) = dict.find_child("Databases") {
        for db in &dbs.children {
            if let Some(p) = db.find_child("PathData") {
                paths.push(p.text.trim().to_string());
            }
        }
    }

    if let Some(dss) = dict.find_child("DataSources") {
        for ds_node in &dss.children {
            let name = ds_node
                .find_child("Name")
                .map(|n| n.text.trim().to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| ds_node.tag.clone());

            let mut columns = Vec::new();
            if let Some(cols) = ds_node.find_child("Columns") {
                for col in &cols.children {
                    // Format: "colname,System.String"
                    let parts: Vec<&str> = col.text.trim().split(',').collect();
                    if !parts[0].trim().is_empty() {
                        columns.push(Column {
                            name: parts[0].trim().to_string(),
                            data_type: parts.get(1).map(|t| t.trim().to_string()).unwrap_or_else(|| "System.String".into()),
                        });
                    }
                }
            }

            if !report.dictionary.data_sources.iter().any(|d| d.name == name) {
                report.dictionary.data_sources.push(DataSource {
                    name: name.clone(),
                    alias: name,
                    columns,
                    rows: Vec::new(),
                });
            }
        }
    }

    merge_data_paths(report, &paths, base_dir);
}

// ---------------------------------------------------------------------------
// Pages and components
// ---------------------------------------------------------------------------

fn build_pages(report: &mut Report, pages: &Value, unit: f32) {
    let mut list: Vec<&Value> = match pages {
        Value::Array(a) => a.iter().collect(),
        Value::Object(m) => {
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<usize>().unwrap_or(usize::MAX));
            entries.into_iter().map(|(_, v)| v).collect()
        }
        _ => Vec::new(),
    };

    report.pages.clear();
    for pv in list.drain(..) {
        report.pages.push(build_page(pv, unit));
    }
    if report.pages.is_empty() {
        report.pages.push(Page::default());
    }
}

fn build_page(pv: &Value, unit: f32) -> Page {
    let mut page = Page::default();
    let name = str_of(pv, "Name");
    if !name.is_empty() {
        page.name = name.to_string();
    }

    // Stimulsoft stores the size as shown on screen, so a landscape page has width > height.
    // Store the upright size and let the orientation flag say which way it is shown.
    let (w, h) = match (num_of(pv, "PageWidth"), num_of(pv, "PageHeight")) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => (w * unit, h * unit),
        _ => (page.width, page.height),
    };
    page.width = w.min(h);
    page.height = w.max(h);
    page.orientation = if w > h { Orientation::Landscape } else { Orientation::Portrait };

    if let Some(wm) = pv.get("Watermark").and_then(|x| serde_json::from_value(x.clone()).ok()) {
        page.watermark = Some(wm);
    }
    if let Some(gs) = pv.get("Guides").and_then(|x| serde_json::from_value(x.clone()).ok()) {
        page.guides = gs;
    }
    if let Some(m) = pv.get("Margins") {
        let (l, r, t, b) = match m {
            Value::Object(_) => (num_of(m, "Left"), num_of(m, "Right"), num_of(m, "Top"), num_of(m, "Bottom")),
            Value::String(s) => {
                let p: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect();
                if p.len() == 4 {
                    (Some(p[0]), Some(p[1]), Some(p[2]), Some(p[3]))
                } else {
                    (None, None, None, None)
                }
            }
            _ => (None, None, None, None),
        };
        let mm = |v: Option<f32>, fallback: f32| (v.unwrap_or(fallback / unit) * unit).min(50.0);
        page.margins.left = mm(l, page.margins.left);
        page.margins.right = mm(r, page.margins.right);
        page.margins.top = mm(t, page.margins.top);
        page.margins.bottom = mm(b, page.margins.bottom);
    }

    if let Some(comps) = pv.get("Components") {
        collect_components(&mut page, comps, (0.0, 0.0), None, unit);
    }
    page
}

/// Adds every component under `comps`. `origin` is the absolute position of the parent container
/// (page origin, band origin or panel origin), so child positions are made absolute here.
fn collect_components(page: &mut Page, comps: &Value, origin: (f32, f32), parent_band: Option<String>, unit: f32) {
    let mut items: Vec<&Value> = match comps {
        Value::Object(m) => m.values().collect(),
        Value::Array(a) => a.iter().collect(),
        _ => return,
    };
    // Containers first, so a panel's background is drawn beneath its children.
    items.sort_by_key(|v| if str_of(v, "Ident") == "StiPanel" { 0 } else { 1 });

    for cv in items {
        let ident = str_of(cv, "Ident");

        if let Some(band_type) = ident_to_band_type(ident) {
            let rect = parse_rect(cv.get("ClientRectangle"), unit);
            let band_id = Uuid::new_v4().to_string();
            let ds = str_of(cv, "DataSourceName");
            page.bands.push(Band {
                id: band_id.clone(),
                name: {
                    let n = str_of(cv, "Name");
                    if n.is_empty() { "Band".to_string() } else { n.to_string() }
                },
                band_type,
                top: rect.y,
                height: rect.h,
                data_source: if ds.is_empty() { None } else { Some(ds.to_string()) },
                group_by: group_condition(cv),
            });
            // Band children are positioned from the band's top-left, inside the page margins.
            let band_origin = (page.margins.left + rect.x, rect.y);
            if let Some(children) = cv.get("Components") {
                collect_components(page, children, band_origin, Some(band_id), unit);
            }
            continue;
        }

        // Unsupported component types are skipped, together with anything nested in them.
        let Some(mut comp) = parse_component(cv, unit) else { continue };
        comp.rect.x += origin.0;
        comp.rect.y += origin.1;
        comp.parent_band_id = parent_band.clone();
        let child_origin = (comp.rect.x, comp.rect.y);
        // Zero-size boxes draw nothing in Stimulsoft (but their children still count). Lines are
        // allowed to be zero in one direction.
        let is_line = comp.comp_type == ComponentType::Line;
        let has_area = comp.rect.w > 0.0 && comp.rect.h > 0.0;
        if has_area || is_line {
            page.components.push(comp);
        }

        if let Some(children) = cv.get("Components") {
            collect_components(page, children, child_origin, parent_band.clone(), unit);
        }
    }
}

fn parse_component(cv: &Value, unit: f32) -> Option<Component> {
    let ident = str_of(cv, "Ident");
    let (comp_type, is_panel) = match ident {
        "StiText" | "StiRichText" | "StiTableCell" => (ComponentType::Text, false),
        // A panel is a box: draw it as an empty text cell with its own fill and borders.
        "StiPanel" => (ComponentType::Text, true),
        "StiImage" => (ComponentType::Image, false),
        "StiShape" | "StiRectanglePrimitive" => (ComponentType::Shape, false),
        "StiHorizontalLinePrimitive" | "StiVerticalLinePrimitive" => (ComponentType::Line, false),
        "StiBarCode" => (ComponentType::Barcode, false),
        "StiCheckBox" => (ComponentType::CheckBox, false),
        "StiTable" => (ComponentType::Table, false),
        _ => return None,
    };

    let mut comp = Component::default();
    comp.id = Uuid::new_v4().to_string();
    comp.comp_type = comp_type;
    comp.name = {
        let n = str_of(cv, "Name");
        if n.is_empty() { ident.trim_start_matches("Sti").to_string() } else { n.to_string() }
    };
    comp.rect = parse_rect(cv.get("ClientRectangle"), unit);

    if is_panel {
        // A panel is only a box: clear the default text so it does not show "Text Label".
        comp.text = String::new();
    } else {
        comp.text = match cv.get("Text") {
            Some(Value::String(s)) => s.clone(),
            Some(obj @ Value::Object(_)) => str_of(obj, "Value").to_string(),
            _ => String::new(),
        };
    }

    if let Some(font) = cv.get("Font").and_then(|x| x.as_str()) {
        apply_font(&mut comp, font);
    }

    match str_of(cv, "HorAlignment") {
        "Center" => comp.align_h = AlignH::Center,
        "Right" => comp.align_h = AlignH::Right,
        "Width" | "Justify" => comp.align_h = AlignH::Justify,
        "Left" => comp.align_h = AlignH::Left,
        _ => {}
    }
    match str_of(cv, "VertAlignment") {
        "Center" => comp.align_v = AlignV::Center,
        "Bottom" => comp.align_v = AlignV::Bottom,
        "Top" => comp.align_v = AlignV::Top,
        _ => {}
    }

    if let Some(c) = cv.get("Brush").and_then(|x| x.as_str()).and_then(parse_brush) {
        comp.bg_color = c;
    }
    // Stored as "solid:R,G,B,A" in JSON (or a colour name / [A:R:G:B] in XML). The prefix must go first.
    if let Some(c) = cv
        .get("TextBrush")
        .and_then(|x| x.as_str())
        .and_then(|s| parse_color(s.trim().strip_prefix("solid:").unwrap_or(s.trim())))
    {
        comp.text_color = c;
    }
    if let Some(b) = cv.get("Border").and_then(|x| x.as_str()) {
        apply_border(&mut comp, b);
    }
    // Stimulsoft's default for WordWrap is off; files only store it when it is on.
    comp.word_wrap = flag_of(cv, "WordWrap");
    if let Some(checked) = cv.get("Checked").and_then(|x| x.as_bool()) {
        comp.checked = checked;
    }
    comp.allow_html_tags = flag_of(cv, "AllowHtmlTags");
    comp.conditions = cv.get("Conditions").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
    // Picture data is stored as base64 (JSON string, or the same text in XML).
    if let Some(b64) = cv.get("ImageBytes").and_then(|x| x.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        use base64::Engine;
        comp.image_data = base64::engine::general_purpose::STANDARD.decode(b64).ok();
    }
    Some(comp)
}

/// The group condition of a group band, as Source.Column (braces removed).
fn group_condition(cv: &Value) -> Option<String> {
    let raw = match cv.get("Condition") {
        Some(Value::String(s)) => s.clone(),
        Some(obj) => str_of(obj, "Value").to_string(),
        None => String::new(),
    };
    let text = raw.trim().trim_start_matches('{').trim_end_matches('}').trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn ident_to_band_type(ident: &str) -> Option<BandType> {
    if !ident.ends_with("Band") {
        return None;
    }
    if ident.contains("ReportTitle") {
        Some(BandType::ReportTitle)
    } else if ident.contains("PageHeader") {
        Some(BandType::PageHeader)
    } else if ident.contains("PageFooter") {
        Some(BandType::PageFooter)
    } else if ident.contains("ReportSummary") {
        Some(BandType::ReportSummary)
    } else if ident.contains("Header") {
        Some(BandType::Header)
    } else if ident.contains("Footer") {
        Some(BandType::Footer)
    } else if ident.contains("Data") {
        Some(BandType::DataBand)
    } else {
        None
    }
}

fn read_meta(v: &Value, report: &mut Report) {
    if let Some(name) = v.get("ReportName").and_then(|x| x.as_str()) {
        report.name = name.to_string();
    }
    if let Some(alias) = v.get("ReportAlias").and_then(|x| x.as_str()) {
        report.alias = alias.to_string();
    }
    if let Some(ver) = v.get("ReportVersion").and_then(|x| x.as_str()) {
        report.version = ver.to_string();
    }
    if let Some(guid) = v.get("ReportGuid").and_then(|x| x.as_str()) {
        report.guid = guid.to_string();
    }
}

// ---------------------------------------------------------------------------
// XML -> JSON-shaped values
// ---------------------------------------------------------------------------

/// Converts an XML element into the JSON shape used by the readers above.
/// A component element has a `type` attribute (for example "Text"), which becomes `Ident: "StiText"`.
fn xml_object(node: &XmlElement) -> Map<String, Value> {
    let mut m = Map::new();
    if let Some(t) = node.attributes.get("type") {
        m.insert("Ident".to_string(), Value::String(format!("Sti{t}")));
    }
    for child in &node.children {
        if child.tag == "Components" {
            let mut comps = Map::new();
            for item in &child.children {
                comps.insert(item.tag.clone(), Value::Object(xml_object(item)));
            }
            m.insert("Components".to_string(), Value::Object(comps));
        } else if child.children.is_empty() {
            m.insert(child.tag.clone(), Value::String(child.text.clone()));
        } else {
            m.insert(child.tag.clone(), Value::Object(xml_object(child)));
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints a summary for every .mrt in the folder named by `NUSA_MRT_DIR`.
    /// Skipped when the variable is not set, so it does not depend on any machine.
    #[test]
    fn summarise_sample_folder() {
        let Ok(dir) = std::env::var("NUSA_MRT_DIR") else { return };
        let mut files: Vec<_> = std::fs::read_dir(&dir).expect("sample dir").flatten().map(|e| e.path()).collect();
        files.sort();
        for path in files.iter().filter(|p| p.extension().map_or(false, |e| e == "mrt")) {
            match MrtHandler::load_from_file(path) {
                Ok(r) => {
                    let pages: Vec<String> = r
                        .pages
                        .iter()
                        .map(|p| {
                            let (w, h) = p.effective_dimensions();
                            let o = match p.orientation { Orientation::Landscape => "L", Orientation::Portrait => "P" };
                            format!("{}:{:.0}x{:.0}{}[b{} c{}]", p.name, w, h, o, p.bands.len(), p.components.len())
                        })
                        .collect();
                    println!(
                        "OK  {:<42} ds={:<2} {}",
                        path.file_name().unwrap().to_string_lossy(),
                        r.dictionary.data_sources.len(),
                        pages.join(" | ")
                    );
                }
                Err(e) => println!("ERR {:<42} {e}", path.file_name().unwrap().to_string_lossy()),
            }
        }
    }
}

