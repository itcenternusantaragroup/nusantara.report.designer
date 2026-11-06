use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use serde_json::{json, Value};

use super::report::*;

pub struct MrtHandler;

impl MrtHandler {
    /// Save a report to a .mrt file (Stimulsoft standard format)
    pub fn save_to_file<P: AsRef<Path>>(report: &Report, path: P) -> Result<(), String> {
        let json_value = Self::to_stimulsoft_json(report)?;
        let json_str = serde_json::to_string_pretty(&json_value)
            .map_err(|e| format!("JSON serialization error: {e}"))?;

        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create file: {e}"))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| format!("Failed to write file: {e}"))?;

        Ok(())
    }

    /// Load a report from a .mrt file (auto-detects XML or JSON Stimulsoft format)
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Report, String> {
        let mut file = File::open(&path)
            .map_err(|e| format!("Failed to open file: {e}"))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("Failed to read file: {e}"))?;

        let mut report = Self::from_mrt_content_at(&content, path.as_ref().parent())?;
        report.file_path = Some(path.as_ref().to_string_lossy().to_string());
        Ok(report)
    }

    /// Auto-detect and parse MRT content whether it's XML or JSON
    pub fn from_mrt_content(content: &str) -> Result<Report, String> {
        Self::from_mrt_content_at(content, None)
    }

    /// Like `from_mrt_content`, with the folder the content was read from (used to find data files).
    pub fn from_mrt_content_at(content: &str, base_dir: Option<&Path>) -> Result<Report, String> {
        let trimmed = content.trim_start_matches('\u{feff}').trim_start();
        if trimmed.starts_with("<?xml") || trimmed.starts_with("<StiSerializer") {
            Self::from_stimulsoft_xml_str(trimmed, base_dir)
        } else if trimmed.starts_with('{') {
            Self::from_stimulsoft_json_str(trimmed, base_dir)
        } else {
            Err(format!("Unsupported file format: content starts with {:?}", &trimmed[..trimmed.len().min(30)]))
        }
    }

    /// Convert Report to Stimulsoft Reports JSON structure (using standard Centimeter units)
    pub fn to_stimulsoft_json(report: &Report) -> Result<Value, String> {
        let mut pages_map = BTreeMap::new();

        for (p_idx, page) in report.pages.iter().enumerate() {
            let mut page_components_map = BTreeMap::new();
            let mut comp_counter = 0;

            // In Stimulsoft standard, units are in Centimeters (1 cm = 10 mm)
            // Stimulsoft stores the size as shown, so write the effective (oriented) size.
            let (eff_w, eff_h) = page.effective_dimensions();
            let page_w_cm = eff_w / 10.0;
            let page_h_cm = eff_h / 10.0;
            let m_left_cm = page.margins.left / 10.0;
            let m_right_cm = page.margins.right / 10.0;
            let m_top_cm = page.margins.top / 10.0;
            let m_bottom_cm = page.margins.bottom / 10.0;
            let printable_w_cm = page.printable_width() / 10.0;

            for band in &page.bands {
                let mut band_comps_map = BTreeMap::new();
                let mut band_comp_counter = 0;

                for comp in &page.components {
                    if comp.parent_band_id.as_deref() == Some(&band.id) {
                        // Relative to band top in cm
                        let rx_cm = (comp.rect.x - page.margins.left).max(0.0) / 10.0;
                        let ry_cm = (comp.rect.y - band.top).max(0.0) / 10.0;
                        let rw_cm = comp.rect.w / 10.0;
                        let rh_cm = comp.rect.h / 10.0;
                        let comp_json = Self::component_to_json(comp, rx_cm, ry_cm, rw_cm, rh_cm);
                        band_comps_map.insert(band_comp_counter.to_string(), comp_json);
                        band_comp_counter += 1;
                    }
                }

                let band_top_cm = band.top / 10.0;
                let band_height_cm = band.height / 10.0;

                let band_json = json!({
                    "Ident": band.band_type.sti_ident(),
                    "Name": band.name,
                    "ClientRectangle": format!("0,{band_top_cm:.2},{printable_w_cm:.2},{band_height_cm:.2}"),
                    "Interaction": { "Ident": "StiInteraction" },
                    "Border": ";;;;;;;solid:0,0,0",
                    "Brush": "solid:Transparent",
                    "DataSourceName": band.data_source.clone().unwrap_or_default(),
                    "Components": band_comps_map
                });

                page_components_map.insert(comp_counter.to_string(), band_json);
                comp_counter += 1;
            }

            // Standalone components (not belonging to any band)
            for comp in &page.components {
                if comp.parent_band_id.is_none() {
                    let rx_cm = comp.rect.x / 10.0;
                    let ry_cm = comp.rect.y / 10.0;
                    let rw_cm = comp.rect.w / 10.0;
                    let rh_cm = comp.rect.h / 10.0;
                    let comp_json = Self::component_to_json(comp, rx_cm, ry_cm, rw_cm, rh_cm);
                    page_components_map.insert(comp_counter.to_string(), comp_json);
                    comp_counter += 1;
                }
            }

            let page_json = json!({
                "Ident": "StiPage",
                "Name": page.name,
                "Guid": page.id,
                "PageWidth": page_w_cm,
                "PageHeight": page_h_cm,
                "Orientation": match page.orientation {
                    Orientation::Portrait => "Portrait",
                    Orientation::Landscape => "Landscape",
                },
                "Margins": {
                    "Left": m_left_cm,
                    "Right": m_right_cm,
                    "Top": m_top_cm,
                    "Bottom": m_bottom_cm,
                },
                "Components": page_components_map
            });

            pages_map.insert(p_idx.to_string(), page_json);
        }

        // Data Sources in Dictionary
        let mut ds_map = BTreeMap::new();
        for (d_idx, ds) in report.dictionary.data_sources.iter().enumerate() {
            let mut cols_map = BTreeMap::new();
            for (c_idx, col) in ds.columns.iter().enumerate() {
                cols_map.insert(c_idx.to_string(), json!({
                    "Name": col.name,
                    "Index": -1,
                    "NameInSource": col.name,
                    "Alias": col.name,
                    "Type": col.data_type,
                }));
            }
            ds_map.insert(d_idx.to_string(), json!({
                "Ident": "StiDataTableSource",
                "Name": ds.name,
                "Alias": ds.alias,
                "Columns": cols_map,
                "Rows": ds.rows,
            }));
        }

        // Variables in Dictionary
        let mut vars_map = BTreeMap::new();
        for (v_idx, var) in report.dictionary.variables.iter().enumerate() {
            vars_map.insert(v_idx.to_string(), json!({
                "Ident": "StiVariable",
                "Name": var.name,
                "Alias": var.alias,
                "Type": var.var_type,
                "Value": var.value,
            }));
        }

        let doc = json!({
            "ReportVersion": report.version,
            "ReportGuid": report.guid,
            "ReportName": report.name,
            "ReportAlias": report.alias,
            "ReportFile": report.file_path.clone().unwrap_or_else(|| "report.mrt".to_string()),
            "ReportCreated": "/Date(1656000000000)/",
            "ReportChanged": "/Date(1656000000000)/",
            "EngineVersion": "EngineV2",
            "CalculationMode": "Compilation",
            "ReportUnit": "Centimeters",
            "Pages": pages_map,
            "Dictionary": {
                "DataSources": ds_map,
                "Variables": vars_map,
            }
        });

        Ok(doc)
    }

    fn component_to_json(comp: &Component, x_cm: f32, y_cm: f32, w_cm: f32, h_cm: f32) -> Value {
        let font_flags = match (comp.bold, comp.italic) {
            (true, true) => "Bold,Italic",
            (true, false) => "Bold",
            (false, true) => "Italic",
            (false, false) => "Regular",
        };
        let font_str = format!("{},{},{}", comp.font_family, comp.font_size, font_flags);

        let hor_align = match comp.align_h {
            AlignH::Left => "Left",
            AlignH::Center => "Center",
            AlignH::Right => "Right",
            AlignH::Justify => "Width",
        };

        let vert_align = match comp.align_v {
            AlignV::Top => "Top",
            AlignV::Center => "Center",
            AlignV::Bottom => "Bottom",
        };

        let brush = if comp.bg_color[3] == 0 {
            "solid:Transparent".to_string()
        } else {
            format!("solid:{},{},{},{}", comp.bg_color[0], comp.bg_color[1], comp.bg_color[2], comp.bg_color[3])
        };

        let text_brush = format!("solid:{},{},{},{}", comp.text_color[0], comp.text_color[1], comp.text_color[2], comp.text_color[3]);

        let border_sides_str = match (comp.border_sides.left, comp.border_sides.top, comp.border_sides.right, comp.border_sides.bottom) {
            (true, true, true, true) => "All",
            (false, false, false, false) => "None",
            _ => "Custom",
        };
        let border_str = format!("{};{};{};;;;solid:{},{},{}",
            border_sides_str,
            comp.border_width,
            "None",
            comp.border_color[0], comp.border_color[1], comp.border_color[2]
        );

        json!({
            "Ident": comp.comp_type.sti_ident(),
            "Name": comp.name,
            "ClientRectangle": format!("{x_cm:.2},{y_cm:.2},{w_cm:.2},{h_cm:.2}"),
            "Interaction": { "Ident": "StiInteraction" },
            "Text": { "Value": comp.text },
            "Font": font_str,
            "Border": border_str,
            "Brush": brush,
            "TextBrush": text_brush,
            "HorAlignment": hor_align,
            "VertAlignment": vert_align,
            "WordWrap": comp.word_wrap,
            "ShapeKind": format!("{:?}", comp.shape_kind),
            "BarCodeType": comp.barcode_format,
            "Checked": comp.checked,
        })
    }

}

/// Lightweight XML node representation for reading Stimulsoft Reports XML templates
#[derive(Debug, Clone)]
pub struct XmlElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<XmlElement>,
    pub text: String,
}

impl XmlElement {
    pub fn find_child(&self, tag: &str) -> Option<&XmlElement> {
        self.children.iter().find(|c| c.tag == tag)
    }

    pub fn parse(xml: &str) -> Result<XmlElement, String> {
        let mut tokens = Self::tokenize(xml)?;
        Self::parse_element(&mut tokens)
    }

    fn tokenize(xml: &str) -> Result<Vec<XmlToken>, String> {
        let mut tokens = Vec::new();
        let bytes = xml.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'<' {
                // Check if comment <!-- -->
                if xml[i..].starts_with("<!--") {
                    if let Some(end) = xml[i..].find("-->") {
                        i += end + 3;
                        continue;
                    }
                }
                // Check if XML declaration <?xml ... ?>
                if xml[i..].starts_with("<?") {
                    if let Some(end) = xml[i..].find("?>") {
                        i += end + 2;
                        continue;
                    }
                }

                // Normal tag
                if let Some(end) = xml[i..].find('>') {
                    let tag_content = &xml[i + 1..i + end];
                    i += end + 1;

                    let tag_trimmed = tag_content.trim();
                    if tag_trimmed.starts_with('/') {
                        tokens.push(XmlToken::EndTag(tag_trimmed[1..].trim().to_string()));
                    } else if tag_trimmed.ends_with('/') {
                        tokens.push(XmlToken::SelfClosingTag(tag_trimmed[..tag_trimmed.len() - 1].trim().to_string()));
                    } else {
                        tokens.push(XmlToken::StartTag(tag_trimmed.to_string()));
                    }
                } else {
                    return Err("Unclosed tag '<'".to_string());
                }
            } else {
                // Text content until next '<'
                let text_start = i;
                while i < bytes.len() && bytes[i] != b'<' {
                    i += 1;
                }
                let raw_text = &xml[text_start..i];
                let decoded = raw_text
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");

                tokens.push(XmlToken::Text(decoded));
            }
        }

        Ok(tokens)
    }

    fn parse_element(tokens: &mut [XmlToken]) -> Result<XmlElement, String> {
        let mut idx = 0;
        Self::parse_element_recursive(tokens, &mut idx)
    }

    fn parse_element_recursive(tokens: &[XmlToken], idx: &mut usize) -> Result<XmlElement, String> {
        while *idx < tokens.len() {
            match &tokens[*idx] {
                XmlToken::Text(t) => {
                    if t.trim().is_empty() {
                        *idx += 1;
                        continue;
                    }
                    *idx += 1;
                }
                XmlToken::StartTag(tag_str) => {
                    let (tag, attrs) = Self::parse_tag_and_attrs(tag_str);
                    *idx += 1;

                    let mut children = Vec::new();
                    let mut text_acc = String::new();

                    while *idx < tokens.len() {
                        match &tokens[*idx] {
                            XmlToken::EndTag(end_tag) => {
                                if end_tag == &tag {
                                    *idx += 1;
                                    break;
                                } else {
                                    *idx += 1;
                                    break;
                                }
                            }
                            XmlToken::Text(t) => {
                                text_acc.push_str(t);
                                *idx += 1;
                            }
                            XmlToken::StartTag(_) | XmlToken::SelfClosingTag(_) => {
                                let child = Self::parse_element_recursive(tokens, idx)?;
                                children.push(child);
                            }
                        }
                    }

                    return Ok(XmlElement {
                        tag,
                        attributes: attrs,
                        children,
                        text: text_acc,
                    });
                }
                XmlToken::SelfClosingTag(tag_str) => {
                    let (tag, attrs) = Self::parse_tag_and_attrs(tag_str);
                    *idx += 1;
                    return Ok(XmlElement {
                        tag,
                        attributes: attrs,
                        children: Vec::new(),
                        text: String::new(),
                    });
                }
                XmlToken::EndTag(_) => {
                    *idx += 1;
                }
            }
        }

        Err("Unexpected end of XML tokens".to_string())
    }

    fn parse_tag_and_attrs(s: &str) -> (String, HashMap<String, String>) {
        let mut parts = s.split_whitespace();
        let tag = parts.next().unwrap_or("").to_string();
        let mut attrs = HashMap::new();

        let rest = s[tag.len()..].trim();
        let mut chars = rest.chars().peekable();

        while chars.peek().is_some() {
            // Skip the whitespace between attributes; otherwise the name is read as empty and
            // the next attribute (e.g. type="Text") is skipped along with it.
            while chars.peek().map_or(false, |c| c.is_whitespace()) {
                chars.next();
            }
            if chars.peek().is_none() {
                break;
            }
            // Read attr name
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '=' || c.is_whitespace() {
                    break;
                }
                name.push(c);
                chars.next();
            }

            // Skip until '='
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == '=' {
                    break;
                }
            }

            // Skip until quote
            let mut quote = '"';
            while let Some(&c) = chars.peek() {
                if c == '"' || c == '\'' {
                    quote = c;
                    chars.next();
                    break;
                }
                chars.next();
            }

            // Read value until matching quote
            let mut val = String::new();
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == quote {
                    break;
                }
                val.push(c);
            }

            if !name.is_empty() {
                attrs.insert(name, val);
            }
        }

        (tag, attrs)
    }
}

enum XmlToken {
    StartTag(String),
    EndTag(String),
    SelfClosingTag(String),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::dictionary::Dictionary;

    #[test]
    fn test_report_mrt_roundtrip() {
        let original_report = Report::default();
        let json_value = MrtHandler::to_stimulsoft_json(&original_report).expect("Serialization to Stimulsoft JSON failed");
        let json_str = serde_json::to_string_pretty(&json_value).expect("Pretty print failed");

        let loaded_report = MrtHandler::from_stimulsoft_json_str(&json_str, None).expect("Deserialization failed");

        assert_eq!(loaded_report.name, original_report.name);
        assert_eq!(loaded_report.pages.len(), original_report.pages.len());
    }

    #[test]
    fn test_file_save_and_load() {
        let report = Report::default();
        let temp_path = std::env::temp_dir().join("test_roundtrip.mrt");

        MrtHandler::save_to_file(&report, &temp_path).expect("Save to file failed");
        let loaded = MrtHandler::load_from_file(&temp_path).expect("Load from file failed");

        assert_eq!(loaded.name, report.name);
        assert_eq!(loaded.pages[0].components.len(), report.pages[0].components.len());

        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_load_real_surat_jalan_xml() {
        let xml_path = "D:\\Project\\PHP\\hrms.nbi\\assets\\mrt\\surat_jalan.mrt";
        if Path::new(xml_path).exists() {
            let loaded = MrtHandler::load_from_file(xml_path).expect("Failed to load real surat_jalan.mrt");
            println!("Loaded surat_jalan.mrt: {} components on page", loaded.pages[0].components.len());
            assert!(!loaded.pages[0].components.is_empty());
        }

        let memo_path = "D:\\Project\\PHP\\hrms.nbi\\assets\\mrt\\report_internalmemo.mrt";
        if Path::new(memo_path).exists() {
            let loaded = MrtHandler::load_from_file(memo_path).expect("Failed to load report_internalmemo.mrt");
            println!("Loaded report_internalmemo.mrt: {} components on page", loaded.pages[0].components.len());
            assert!(!loaded.pages[0].components.is_empty());
        }

        let letter_path = "D:\\Project\\PHP\\hrms.nbi\\assets\\mrt\\assignment_letter.mrt";
        if Path::new(letter_path).exists() {
            let loaded = MrtHandler::load_from_file(letter_path).expect("Failed to load assignment_letter.mrt");
            println!("Loaded assignment_letter.mrt: {} components on page", loaded.pages[0].components.len());
            assert!(!loaded.pages[0].components.is_empty());
        }
    }

    #[test]
    fn test_load_real_form_pbk_json() {
        let json_path = "D:\\Project\\PHP\\hrms.nbi\\assets\\mrt\\form_pbk1.mrt";
        if Path::new(json_path).exists() {
            let loaded = MrtHandler::load_from_file(json_path).expect("Failed to load real form_pbk1.mrt");
            println!("Loaded form_pbk1.mrt: {} components on page", loaded.pages[0].components.len());
            assert!(!loaded.pages[0].components.is_empty());
        }
    }

    #[test]
    fn test_load_json_data_source() {
        let json_path = "D:\\Project\\PHP\\hrms.nbi\\assets\\json\\ijin_karyawan.json";
        if Path::new(json_path).exists() {
            let sources = Dictionary::load_from_json_file(json_path).expect("Failed to load ijin_karyawan.json");
            println!("Loaded sources: {} tables", sources.len());
            assert!(!sources.is_empty());
            assert!(!sources[0].columns.is_empty());
        }
    }
}
