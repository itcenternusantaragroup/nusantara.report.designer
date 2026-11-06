use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use crate::model::expr::EvalContext;
use crate::model::richtext::parse_runs;
use crate::model::*;
use super::designer::{AppMode, ReportDesignerApp};
use super::icons::{self, Icon};
use super::richtext as rt;
use super::theme::{self, DESK, PAPER, RULE};

/// Space between printed sheets, in screen pixels.
const SHEET_GAP: f32 = 24.0;

pub struct PreviewView;

/// A component placed on one printed sheet. `comp.rect` is its position on that sheet (mm).
#[derive(Clone)]
struct Item {
    comp: Component,
    /// Row index per data source for the band this item came from.
    rows: HashMap<String, usize>,
}

/// One printed page of the report.
struct Sheet {
    page_idx: usize,
    items: Vec<Item>,
}

impl PreviewView {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::TopBottomPanel::top("preview_toolbar")
            .frame(Frame::none().fill(PAPER).inner_margin(Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if icons::chip(ui, Some(Icon::Design), "Back to design", true, false).clicked() {
                        app.app_mode = AppMode::Design;
                    }
                    ui.separator();
                    if icons::chip(ui, None, "Export HTML...", true, false).clicked() {
                        Self::export_html_dialog(app);
                    }
                    if icons::chip(ui, None, "Export text...", true, false).clicked() {
                        Self::export_text_dialog(app);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+").clicked() {
                            app.zoom = (app.zoom + 0.15).min(2.5);
                            app.fit_to_window = false;
                        }
                        ui.label(egui::RichText::new(format!("{:.0}%", app.zoom * 100.0)).color(theme::MUTED));
                        if ui.button("-").clicked() {
                            app.zoom = (app.zoom - 0.15).max(0.5);
                            app.fit_to_window = false;
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).fill(DESK))
            .show(ctx, |ui| {
                let mut area = egui::ScrollArea::both().auto_shrink([false, false]);
                if let Some(n) = app.preview_sheet {
                    area = area.vertical_scroll_offset(sheet_offset(app, n));
                }
                area.show(ui, |ui| Self::render(app, ui));
            });
    }

    fn render(app: &ReportDesignerApp, ui: &mut egui::Ui) {
        let sheets = build_sheets(app);
        let total = sheets.len();
        let vars = base_vars(app);
        let mm_px = 3.78 * app.zoom;

        let max_w = sheets
            .iter()
            .map(|s| page_size(app, s).0 * mm_px)
            .fold(0.0_f32, f32::max);
        let total_h: f32 = sheets.iter().map(|s| page_size(app, s).1 * mm_px + SHEET_GAP).sum::<f32>() + SHEET_GAP;
        let width = max_w + 80.0;

        let (resp, painter) = ui.allocate_painter(Vec2::new(width, total_h), Sense::hover());
        let mut y = resp.rect.min.y + SHEET_GAP;

        for (n, sheet) in sheets.iter().enumerate() {
            let (pw, ph) = page_size(app, sheet);
            let (w_px, h_px) = (pw * mm_px, ph * mm_px);
            let x = resp.rect.min.x + ((width - w_px) / 2.0).max(20.0);
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w_px, h_px));
            draw_paper(&painter, rect);

            let mut sheet_vars = vars.clone();
            sheet_vars.insert("PageNumber".to_string(), (n + 1).to_string());
            sheet_vars.insert("TotalPageCount".to_string(), total.to_string());

            for item in &sheet.items {
                let eval = EvalContext {
                    dictionary: &app.report.dictionary,
                    row_index: &item.rows,
                    vars: &sheet_vars,
                };
                let mut comp = item.comp.clone();
                if matches!(comp.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox) {
                    comp.text = eval.evaluate(&comp.text);
                }
                draw_component(&painter, &comp, rect, mm_px, app.zoom);
            }

            y += h_px + SHEET_GAP;
        }
    }

    fn export_html_dialog(app: &mut ReportDesignerApp) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML document (*.html)", &["html"])
            .set_file_name("report.html")
            .save_file()
        {
            match std::fs::write(&path, sheets_to_html(app)) {
                Ok(()) => app.status_message = format!("Exported HTML report to {}", path.display()),
                Err(e) => app.status_message = format!("Export failed: {e}"),
            }
        }
    }

    fn export_text_dialog(app: &mut ReportDesignerApp) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text file (*.txt)", &["txt"])
            .set_file_name("report.txt")
            .save_file()
        {
            let text = plain_text(app);
            match std::fs::write(&path, text) {
                Ok(()) => app.status_message = format!("Exported text report to {}", path.display()),
                Err(e) => app.status_message = format!("Export failed: {e}"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pagination: place every band of every page on sheets, repeating data bands per row
// and starting a new sheet when a band would run into the page footer.
// ---------------------------------------------------------------------------

fn build_sheets(app: &ReportDesignerApp) -> Vec<Sheet> {
    let mut sheets = Vec::new();
    for (pi, page) in app.report.pages.iter().enumerate() {
        sheets.extend(paginate(app, pi, page));
    }
    sheets
}

fn paginate(app: &ReportDesignerApp, pi: usize, page: &Page) -> Vec<Sheet> {
    let dict = &app.report.dictionary;
    let (_, ph) = page.effective_dimensions();
    let find = |t: BandType| page.bands.iter().find(|b| b.band_type == t);
    let page_header = find(BandType::PageHeader);
    let page_footer = find(BandType::PageFooter);
    let footer_h = page_footer.map_or(0.0, |b| b.height);

    let mut flow = Flow {
        page,
        pi,
        top: page.margins.top,
        header: page_header,
        header_h: page_header.map_or(0.0, |b| b.height),
        limit: ph - page.margins.bottom - footer_h,
        sheets: Vec::new(),
        items: Vec::new(),
        y: page.margins.top,
        body: false,
    };
    flow.new_sheet();

    // Components outside any band print once, on the first sheet of the page.
    for c in page.components.iter().filter(|c| c.parent_band_id.is_none()) {
        flow.items.push(Item { comp: c.clone(), rows: HashMap::new() });
    }

    let mut bands: Vec<&Band> = page.bands.iter().collect();
    bands.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(Ordering::Equal));
    let none = HashMap::new();

    for band in bands {
        match band.band_type {
            BandType::PageHeader | BandType::PageFooter => {}
            BandType::DataBand => {
                // Use the band's own data source; if it has none, use the source its fields refer to.
                let source = band
                    .data_source
                    .as_ref()
                    .and_then(|name| dict.data_sources.iter().find(|d| &d.name == name))
                    .or_else(|| inferred_source(page, band, dict));
                match source {
                    Some(ds) if !ds.rows.is_empty() => {
                        for i in 0..ds.rows.len() {
                            let rows = HashMap::from([(ds.name.clone(), i)]);
                            flow.flow(band, &rows);
                        }
                    }
                    // A band with no data source prints once; an empty source prints the layout once.
                    _ => flow.flow(band, &none),
                }
            }
            _ => flow.flow(band, &none),
        }
    }

    flow.finish(page_footer)
}

struct Flow<'a> {
    page: &'a Page,
    pi: usize,
    top: f32,
    header: Option<&'a Band>,
    header_h: f32,
    /// Lowest y (mm) that body bands may reach on a sheet.
    limit: f32,
    sheets: Vec<Sheet>,
    items: Vec<Item>,
    /// Next free y (mm) for body bands on the current sheet.
    y: f32,
    /// True once a body band has been placed on the current sheet.
    body: bool,
}

impl<'a> Flow<'a> {
    fn new_sheet(&mut self) {
        if !self.items.is_empty() {
            let items = std::mem::take(&mut self.items);
            self.sheets.push(Sheet { page_idx: self.pi, items });
        }
        if let Some(h) = self.header {
            place_into(self.page, h, self.top, &HashMap::new(), &mut self.items);
        }
        self.y = self.top + self.header_h;
        self.body = false;
    }

    fn flow(&mut self, band: &Band, rows: &HashMap<String, usize>) {
        if self.body && self.y + band.height > self.limit {
            self.new_sheet();
        }
        place_into(self.page, band, self.y, rows, &mut self.items);
        self.y += band.height;
        self.body = true;
    }

    fn finish(mut self, page_footer: Option<&Band>) -> Vec<Sheet> {
        if let Some(pf) = page_footer {
            let none = HashMap::new();
            for sheet in &mut self.sheets {
                place_into(self.page, pf, self.limit, &none, &mut sheet.items);
            }
            place_into(self.page, pf, self.limit, &none, &mut self.items);
        }
        let items = std::mem::take(&mut self.items);
        self.sheets.push(Sheet { page_idx: self.pi, items });
        self.sheets
    }
}

/// The first data source named in the expressions of a band's components (`{Source.Column}`).
fn inferred_source<'a>(page: &Page, band: &Band, dict: &'a Dictionary) -> Option<&'a DataSource> {
    for comp in page.components.iter().filter(|c| c.parent_band_id.as_deref() == Some(band.id.as_str())) {
        let mut rest = comp.text.as_str();
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else { break };
            let token = rest[open + 1..open + close].trim();
            if let Some((source, _)) = token.split_once('.') {
                if let Some(ds) = dict.data_sources.iter().find(|d| d.name == source) {
                    return Some(ds);
                }
            }
            rest = &rest[open + close + 1..];
        }
    }
    None
}

/// Copies the components of `band` into `items`, moved so the band starts at `y`.
fn place_into(page: &Page, band: &Band, y: f32, rows: &HashMap<String, usize>, items: &mut Vec<Item>) {
    let dy = y - band.top;
    for c in page.components.iter().filter(|c| c.parent_band_id.as_deref() == Some(band.id.as_str())) {
        let mut comp = c.clone();
        comp.rect.y += dy;
        items.push(Item { comp, rows: rows.clone() });
    }
}

/// Vertical position (screen pixels, from the top of the preview) where sheet `n` starts.
fn sheet_offset(app: &ReportDesignerApp, n: usize) -> f32 {
    let mm_px = 3.78 * app.zoom;
    let mut y = SHEET_GAP;
    for sheet in build_sheets(app).iter().take(n) {
        y += page_size(app, sheet).1 * mm_px + SHEET_GAP;
    }
    y
}

fn page_size(app: &ReportDesignerApp, sheet: &Sheet) -> (f32, f32) {
    app.report.pages[sheet.page_idx].effective_dimensions()
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn draw_paper(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect.translate(Vec2::new(0.0, 2.0)).expand(1.0), 2.0, Color32::from_black_alpha(28));
    painter.rect_filled(rect, 0.0, PAPER);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, RULE));
}

fn draw_component(painter: &egui::Painter, comp: &Component, page_rect: Rect, mm_px: f32, zoom: f32) {
    let c_rect = Rect::from_min_size(
        Pos2::new(page_rect.min.x + comp.rect.x * mm_px, page_rect.min.y + comp.rect.y * mm_px),
        Vec2::new(comp.rect.w * mm_px, comp.rect.h * mm_px),
    );
    let color = |c: [u8; 4]| Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]);

    if comp.bg_color[3] > 0 {
        painter.rect_filled(c_rect, 0.0, color(comp.bg_color));
    }
    if comp.border_sides.has_any() {
        let stroke = Stroke::new((comp.border_width * mm_px).max(1.0), color(comp.border_color));
        let s = &comp.border_sides;
        if s.top { painter.line_segment([c_rect.left_top(), c_rect.right_top()], stroke); }
        if s.bottom { painter.line_segment([c_rect.left_bottom(), c_rect.right_bottom()], stroke); }
        if s.left { painter.line_segment([c_rect.left_top(), c_rect.left_bottom()], stroke); }
        if s.right { painter.line_segment([c_rect.right_top(), c_rect.right_bottom()], stroke); }
    }

    match comp.comp_type {
        ComponentType::Text => {
            rt::draw(painter, comp, &comp.text, c_rect, rt::px_per_pt(zoom));
        }
        ComponentType::Barcode => {
            let bc_rect = c_rect.shrink(2.0);
            let bar_w = 2.0 * zoom;
            let mut bx = bc_rect.min.x;
            let mut dark = false;
            while bx < bc_rect.max.x {
                if dark {
                    painter.rect_filled(
                        Rect::from_min_max(Pos2::new(bx, bc_rect.min.y), Pos2::new(bx + bar_w, bc_rect.max.y - 11.0)),
                        0.0,
                        Color32::BLACK,
                    );
                }
                bx += bar_w + if (bx as usize) % 3 == 0 { 1.5 * bar_w } else { 0.5 * bar_w };
                dark = !dark;
            }
            painter.text(
                bc_rect.center_bottom() + Vec2::new(0.0, -1.0),
                egui::Align2::CENTER_BOTTOM,
                &comp.text,
                egui::FontId::monospace(9.0 * zoom),
                Color32::BLACK,
            );
        }
        ComponentType::Line => {
            let w = (comp.border_width * mm_px).max(1.5);
            let (a, b) = if c_rect.width() < c_rect.height() {
                (c_rect.center_top(), c_rect.center_bottom())
            } else {
                (c_rect.left_center(), c_rect.right_center())
            };
            painter.line_segment([a, b], Stroke::new(w, color(comp.border_color)));
        }
        ComponentType::Shape => {
            let fill = color(comp.bg_color);
            let stroke = Stroke::new((comp.border_width * mm_px).max(1.0), color(comp.border_color));
            match comp.shape_kind {
                ShapeKind::Rectangle => {
                    painter.rect(c_rect, 0.0, fill, stroke);
                }
                ShapeKind::RoundedRectangle => {
                    painter.rect(c_rect, 6.0 * zoom, fill, stroke);
                }
                ShapeKind::Ellipse => {
                    painter.circle(c_rect.center(), c_rect.width().min(c_rect.height()) * 0.5, fill, stroke);
                }
                ShapeKind::Triangle => {
                    let points = vec![c_rect.center_top(), c_rect.right_bottom(), c_rect.left_bottom()];
                    painter.add(egui::Shape::convex_polygon(points, fill, stroke));
                }
            }
        }
        ComponentType::CheckBox => {
            let side = c_rect.height().min(4.0 * mm_px);
            let box_rect = Rect::from_min_size(
                Pos2::new(c_rect.left(), c_rect.center().y - side / 2.0),
                Vec2::splat(side),
            );
            painter.rect(box_rect, 1.0, Color32::WHITE, Stroke::new(1.0_f32, Color32::BLACK));
            if comp.checked {
                let p = |x: f32, y: f32| box_rect.min + Vec2::new(x * side, y * side);
                let stroke = Stroke::new((side * 0.12).max(1.0), Color32::BLACK);
                painter.line_segment([p(0.2, 0.5), p(0.42, 0.75)], stroke);
                painter.line_segment([p(0.42, 0.75), p(0.82, 0.25)], stroke);
            }
            let text_rect = Rect::from_min_max(Pos2::new(box_rect.right() + 3.0, c_rect.top()), c_rect.max);
            rt::draw(painter, &comp_as_text(comp), &comp.text, text_rect, rt::px_per_pt(zoom));
        }
        // Images and tables cannot be rendered yet; they keep their place on the page.
        ComponentType::Image | ComponentType::Table => {}
    }
}

/// A copy of the component with left alignment, for text placed beside a checkbox.
fn comp_as_text(comp: &Component) -> Component {
    let mut c = comp.clone();
    c.align_h = AlignH::Left;
    c.align_v = AlignV::Center;
    c.word_wrap = false;
    c
}

// ---------------------------------------------------------------------------
// Values shown in the preview
// ---------------------------------------------------------------------------

fn base_vars(app: &ReportDesignerApp) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for v in &app.report.dictionary.variables {
        vars.insert(v.name.clone(), v.value.clone());
    }
    let (y, m, d) = today();
    let today_text = format!("{d:02}/{m:02}/{y}");
    vars.insert("Today".to_string(), today_text.clone());
    vars.insert("Date".to_string(), today_text);
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|x| x.as_secs()).unwrap_or(0) % 86_400;
    vars.insert("Time".to_string(), format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60));
    vars
}

/// Today's date (UTC) as (year, month, day). Std only, so no time zone.
fn today() -> (i64, u32, u32) {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map(|x| x.as_secs() / 86_400).unwrap_or(0) as i64;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn css_color(c: [u8; 4]) -> String {
    format!("rgba({},{},{},{:.3})", c[0], c[1], c[2], c[3] as f32 / 255.0)
}

/// The text of one component as HTML: runs with their styles when HTML tags are allowed,
/// otherwise the escaped text. Line breaks become <br>.
fn text_html(comp: &Component, text: &str) -> String {
    let runs = parse_runs(text, comp.allow_html_tags);
    let mut out = String::new();
    for run in runs {
        let mut style = String::new();
        if run.bold || comp.bold {
            style.push_str("font-weight:bold;");
        }
        if run.italic || comp.italic {
            style.push_str("font-style:italic;");
        }
        if run.underline || comp.underline {
            style.push_str("text-decoration:underline;");
        }
        if let Some(c) = run.color {
            style.push_str(&format!("color:{};", css_color(c)));
        }
        let size = run.size_pt.unwrap_or(comp.font_size * run.size_scale);
        style.push_str(&format!("font-size:{size:.2}pt;"));
        let body = escape_html(&run.text).replace('\n', "<br>");
        out.push_str(&format!("<span style=\"{style}\">{body}</span>"));
    }
    out
}

fn sheets_to_html(app: &ReportDesignerApp) -> String {
    let sheets = build_sheets(app);
    let total = sheets.len();
    let vars = base_vars(app);
    let title = escape_html(&app.report.alias);

    let mut html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>\
         body{{margin:0;padding:24px;background:#f1f3f4;font-family:'Segoe UI',Arial,sans-serif}}\
         .sheet{{position:relative;background:#fff;margin:0 auto 24px;box-shadow:0 1px 4px rgba(0,0,0,.25);overflow:hidden}}\
         .item{{position:absolute;box-sizing:border-box;overflow:hidden;white-space:normal;line-height:1.2}}\
         </style></head><body>"
    );

    for (n, sheet) in sheets.iter().enumerate() {
        let (pw, ph) = page_size(app, sheet);
        html.push_str(&format!("<div class=\"sheet\" style=\"width:{pw:.2}mm;height:{ph:.2}mm\">"));

        let mut sheet_vars = vars.clone();
        sheet_vars.insert("PageNumber".to_string(), (n + 1).to_string());
        sheet_vars.insert("TotalPageCount".to_string(), total.to_string());

        for item in &sheet.items {
            let comp = &item.comp;
            let eval = EvalContext {
                dictionary: &app.report.dictionary,
                row_index: &item.rows,
                vars: &sheet_vars,
            };
            let text = if matches!(comp.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox) {
                eval.evaluate(&comp.text)
            } else {
                String::new()
            };

            let mut style = format!(
                "left:{:.2}mm;top:{:.2}mm;width:{:.2}mm;height:{:.2}mm;",
                comp.rect.x, comp.rect.y, comp.rect.w, comp.rect.h
            );
            if comp.bg_color[3] > 0 {
                style.push_str(&format!("background:{};", css_color(comp.bg_color)));
            }
            let border = format!("{:.2}mm solid {}", comp.border_width, css_color(comp.border_color));
            let s = &comp.border_sides;
            if s.top { style.push_str(&format!("border-top:{border};")); }
            if s.bottom { style.push_str(&format!("border-bottom:{border};")); }
            if s.left { style.push_str(&format!("border-left:{border};")); }
            if s.right { style.push_str(&format!("border-right:{border};")); }
            style.push_str(&format!("color:{};", css_color(comp.text_color)));
            style.push_str(match comp.align_h {
                AlignH::Center => "text-align:center;",
                AlignH::Right => "text-align:right;",
                _ => "text-align:left;",
            });
            style.push_str(match comp.align_v {
                AlignV::Center => "display:flex;flex-direction:column;justify-content:center;",
                AlignV::Bottom => "display:flex;flex-direction:column;justify-content:flex-end;",
                _ => "",
            });

            let inner = match comp.comp_type {
                ComponentType::Text => text_html(comp, &text),
                ComponentType::Barcode => escape_html(&text),
                ComponentType::CheckBox => format!("{} {}", if comp.checked { "[x]" } else { "[ ]" }, text_html(comp, &text)),
                _ => String::new(),
            };
            if matches!(comp.comp_type, ComponentType::Shape) && comp.shape_kind == ShapeKind::Ellipse {
                style.push_str("border-radius:50%;");
            }
            html.push_str(&format!("<div class=\"item\" style=\"{style}\">{inner}</div>"));
        }
        html.push_str("</div>");
    }

    html.push_str("</body></html>");
    html
}

fn plain_text(app: &ReportDesignerApp) -> String {
    let mut txt = String::new();
    let rule = "=".repeat(88);
    txt.push_str(&format!("{rule}\n{}\n{rule}\n", app.report.alias.to_uppercase()));
    if let Some(ds) = app.report.dictionary.data_sources.first() {
        for col in &ds.columns {
            txt.push_str(&format!("{:<15} ", col.name));
        }
        txt.push('\n');
        for row in &ds.rows {
            for col in &ds.columns {
                let val = row.get(&col.name).cloned().unwrap_or_default();
                txt.push_str(&format!("{:<15} ", val));
            }
            txt.push('\n');
        }
    }
    txt.push_str(&rule);
    txt.push('\n');
    txt
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints every printed sheet of the report named by `NUSA_PREVIEW_FILE`, and every
    /// expression that is still not evaluated. Skipped when the variable is not set.
    #[test]
    fn summarise_preview_sheets() {
        let Ok(path) = std::env::var("NUSA_PREVIEW_FILE") else { return };
        let app = ReportDesignerApp::new(None, Some(path));
        let sheets = build_sheets(&app);
        let vars = base_vars(&app);
        println!("sheets={}", sheets.len());

        let mut unresolved: Vec<String> = Vec::new();
        for (n, sheet) in sheets.iter().enumerate() {
            let (w, h) = page_size(&app, sheet);
            let mut sheet_vars = vars.clone();
            sheet_vars.insert("PageNumber".to_string(), (n + 1).to_string());
            sheet_vars.insert("TotalPageCount".to_string(), sheets.len().to_string());
            let texts = sheet.items.iter().filter(|i| i.comp.comp_type == ComponentType::Text).count();
            println!(
                "sheet {:>2}: page {} {:.0}x{:.0}mm items={} text={}",
                n + 1,
                sheet.page_idx + 1,
                w,
                h,
                sheet.items.len(),
                texts
            );
            for item in &sheet.items {
                let eval = EvalContext {
                    dictionary: &app.report.dictionary,
                    row_index: &item.rows,
                    vars: &sheet_vars,
                };
                let out = eval.evaluate(&item.comp.text);
                if matches!(item.comp.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox)
                    && out.contains('{')
                {
                    unresolved.push(format!("sheet {}: {}", n + 1, out.chars().take(140).collect::<String>()));
                }
            }
        }

        println!("unresolved expressions: {}", unresolved.len());
        for line in unresolved.iter().take(40) {
            println!("  {line}");
        }
    }
}
