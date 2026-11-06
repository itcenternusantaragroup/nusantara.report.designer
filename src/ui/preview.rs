use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use crate::model::expr::EvalContext;
use crate::model::richtext::parse_paragraphs;
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
pub(super) struct Item {
    pub(super) comp: Component,
    /// Row index per data source for the band this item came from.
    pub(super) rows: HashMap<String, usize>,
}

/// One printed page of the report.
pub(super) struct Sheet {
    pub(super) page_idx: usize,
    pub(super) items: Vec<Item>,
}

impl PreviewView {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        let missing: Vec<Vec<u8>> = app
            .report
            .pages
            .iter()
            .flat_map(|p| p.components.iter())
            .filter_map(|c| c.image_data.as_ref())
            .filter(|b| !app.images.contains_key(&super::images::key(b)))
            .cloned()
            .collect();
        super::images::ensure_loaded(ctx, &mut app.images, &missing);

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
                    if icons::chip(ui, None, "Export PDF...", true, false).clicked() {
                        Self::export_pdf_dialog(app);
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
        let running = RunningTotals::new(&app.report.dictionary);
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
            if let Some(wm) = &app.report.pages[sheet.page_idx].watermark {
                draw_watermark(&painter, wm, rect, app.zoom);
            }

            let mut sheet_vars = vars.clone();
            sheet_vars.insert("PageNumber".to_string(), (n + 1).to_string());
            sheet_vars.insert("TotalPageCount".to_string(), total.to_string());

            for item in &sheet.items {
                let mut item_vars = sheet_vars.clone();
                running.apply(&item.rows, &mut item_vars);
                let eval = EvalContext {
                    dictionary: &app.report.dictionary,
                    row_index: &item.rows,
                    vars: &item_vars,
                };
                let mut comp = item.comp.clone();
                if matches!(comp.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox) {
                    comp.text = eval.evaluate(&comp.text);
                }
                let comp = comp.with_conditions(&comp.text);
                draw_component(&painter, &comp, rect, mm_px, app.zoom, &app.images);
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

    fn export_pdf_dialog(app: &mut ReportDesignerApp) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF document (*.pdf)", &["pdf"])
            .set_file_name("report.pdf")
            .save_file()
        {
            let result = super::pdf_export::sheets_to_pdf(app)
                .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
            match result {
                Ok(()) => app.status_message = format!("Exported PDF report to {}", path.display()),
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

pub(super) fn build_sheets(app: &ReportDesignerApp) -> Vec<Sheet> {
    build_report_sheets(&app.report, 0)
}

/// The pages of `report` laid out as sheets. `depth` counts nested subreports.
fn build_report_sheets(report: &Report, depth: usize) -> Vec<Sheet> {
    let mut sheets = Vec::new();
    for (pi, page) in report.pages.iter().enumerate() {
        sheets.extend(paginate(report, pi, page, depth));
    }
    sheets
}

fn paginate(report: &Report, pi: usize, page: &Page, depth: usize) -> Vec<Sheet> {
    let dict = &report.dictionary;
    let (_, ph) = page.effective_dimensions();
    let find = |t: BandType| page.bands.iter().find(|b| b.band_type == t);
    let page_header = find(BandType::PageHeader);
    let page_footer = find(BandType::PageFooter);
    let footer_h = page_footer.map_or(0.0, |b| b.height);

    // Columns: the printable width is split evenly; very narrow columns fall back to one.
    let print_w = page.printable_width();
    let mut columns = page.columns.max(1);
    let mut col_w = (print_w - page.column_gap * (columns as f32 - 1.0)) / columns as f32;
    if col_w < 10.0 {
        columns = 1;
        col_w = print_w;
    }

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
        columns,
        col: 0,
        col_w,
        gap: page.column_gap,
        left: page.margins.left,
        print_w,
    };
    flow.new_sheet();

    // Components outside any band print once, on the first sheet of the page.
    for c in page.components.iter().filter(|c| c.parent_band_id.is_none() && c.table.is_none() && c.subreport.is_none()) {
        match &c.crosstab {
            // A cross-tab prints as its grid of cells.
            Some(ct) => {
                for cell in crate::model::crosstab::cells(c, ct, &report.dictionary) {
                    flow.items.push(Item { comp: cell, rows: HashMap::new() });
                }
            }
            None => flow.items.push(Item { comp: c.clone(), rows: HashMap::new() }),
        }
    }

    // Bands and page-level tables flow in the order of their top edge.
    let mut order: Vec<(f32, Flowable)> = page.bands.iter().map(|b| (b.top, Flowable::Band(b))).collect();
    order.extend(
        page.components
            .iter()
            .filter(|c| c.parent_band_id.is_none() && (c.table.is_some() || c.subreport.is_some()))
            .map(|c| (c.rect.y, if c.subreport.is_some() { Flowable::Sub(c) } else { Flowable::Table(c) })),
    );
    order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let none = HashMap::new();

    // A group (GroupHeader with a valid condition) prints its data bands once per group.
    let group = group_plan(page, dict);
    let mut group_printed = false;

    for (_, item) in order {
        let band = match item {
            Flowable::Band(b) => b,
            Flowable::Table(c) => {
                flow_table(&mut flow, c, &report.dictionary);
                continue;
            }
            Flowable::Sub(c) => {
                flow_subreport(&mut flow, c, report, depth);
                continue;
            }
        };
        match band.band_type {
            // Detail bands print under their parent rows (see flow_details), not here.
            _ if band.detail_of.is_some() => {}
            BandType::PageHeader | BandType::PageFooter | BandType::GroupFooter => {}
            BandType::GroupHeader => {
                if let (Some(plan), false) = (&group, group_printed) {
                    group_printed = true;
                    flow_groups(&mut flow, page, plan);
                }
            }
            BandType::DataBand if group.as_ref().map_or(false, |g| band.data_source.as_deref() == g.source_name()) => {
                // Printed inside the groups.
            }
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
                            flow_details(&mut flow, page, dict, &ds.name, &rows, 0);
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

/// Prints the detail bands of a parent row. A detail band of a relation prints once for every
/// child row whose column matches the parent row; its own details print under each child row.
fn flow_details(flow: &mut Flow, page: &Page, dict: &Dictionary, parent: &str, rows: &HashMap<String, usize>, depth: usize) {
    const MAX_DEPTH: usize = 5;
    if depth >= MAX_DEPTH {
        return;
    }
    let (Some(parent_ds), Some(&parent_row)) = (dict.data_sources.iter().find(|d| d.name == parent), rows.get(parent))
    else {
        return;
    };
    let mut details: Vec<&Band> = page.bands.iter().filter(|b| b.detail_of.is_some()).collect();
    details.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(Ordering::Equal));
    for band in details {
        let Some(rel) = band.detail_of.as_deref().and_then(|name| {
            dict.relations.iter().find(|r| r.name == name && r.parent == parent)
        }) else {
            continue;
        };
        let Some(child) = dict.data_sources.iter().find(|d| d.name == rel.child) else { continue };
        let key = parent_ds
            .rows
            .get(parent_row)
            .and_then(|r| r.get(&rel.parent_column))
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        for (j, row) in child.rows.iter().enumerate() {
            if row.get(&rel.child_column).map(|v| v.trim()) != Some(key.as_str()) {
                continue;
            }
            let mut child_rows = rows.clone();
            child_rows.insert(child.name.clone(), j);
            flow.flow(band, &child_rows);
            flow_details(flow, page, dict, &child.name, &child_rows, depth + 1);
        }
    }
}
/// The grouping of a page: the group header and footer, the data bands that repeat per group,
/// and the sorted row order of the grouped source.
struct GroupPlan<'a> {
    header: &'a Band,
    footer: Option<&'a Band>,
    data_bands: Vec<&'a Band>,
    source: &'a DataSource,
    column: String,
}

impl GroupPlan<'_> {
    fn source_name(&self) -> Option<&str> {
        Some(self.source.name.as_str())
    }
}

/// Builds the group plan when the page has a group header whose condition names a loaded column.
fn group_plan<'a>(page: &'a Page, dict: &'a Dictionary) -> Option<GroupPlan<'a>> {
    let header = page.bands.iter().find(|b| b.band_type == BandType::GroupHeader)?;
    let condition = header.group_by.as_deref()?;
    let (source_name, column) = condition.split_once('.')?;
    let source = dict.data_sources.iter().find(|d| d.name == source_name)?;
    if source.rows.is_empty() {
        return None;
    }
    let footer = page.bands.iter().find(|b| b.band_type == BandType::GroupFooter);
    let mut data_bands: Vec<&Band> = page
        .bands
        .iter()
        .filter(|b| b.band_type == BandType::DataBand && b.data_source.as_deref() == Some(source_name))
        .collect();
    data_bands.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(Ordering::Equal));
    Some(GroupPlan { header, footer, data_bands, source, column: column.to_string() })
}

/// Prints every group: the header, the data bands for each row of the group, then the footer.
/// Rows are sorted by the grouping column, so equal values are next to each other.
fn flow_groups(flow: &mut Flow, page: &Page, plan: &GroupPlan) {
    let _ = page;
    let mut order: Vec<usize> = (0..plan.source.rows.len()).collect();
    let value = |i: usize| plan.source.rows[i].get(&plan.column).map(|s| s.trim().to_string()).unwrap_or_default();
    order.sort_by(|&a, &b| value(a).cmp(&value(b)));

    let mut start = 0;
    while start < order.len() {
        let key = value(order[start]);
        let mut end = start;
        while end + 1 < order.len() && value(order[end + 1]) == key {
            end += 1;
        }
        let first = order[start];
        let last = order[end];
        let name = plan.source.name.clone();

        // Keep the header with its first row; with keep-together, the whole group when it fits on a sheet.
        let rows_here = (end - start + 1) as f32;
        let row_h: f32 = plan.data_bands.iter().map(|b| b.height).sum();
        let footer_h = plan.footer.map_or(0.0, |b| b.height);
        let page_room = flow.limit - (flow.top + flow.header_h);
        let whole = plan.header.height + row_h * rows_here + footer_h;
        let need = if plan.header.keep_together && whole <= page_room { whole } else { plan.header.height + row_h };
        flow.keep(need);

        flow.flow(plan.header, &HashMap::from([(name.clone(), first)]));
        for &row in &order[start..=end] {
            for band in &plan.data_bands {
                flow.flow(band, &HashMap::from([(name.clone(), row)]));
            }
        }
        if let Some(footer) = plan.footer {
            flow.flow(footer, &HashMap::from([(name.clone(), last)]));
        }
        start = end + 1;
    }
}
struct Flow<'a> {
    page: &'a Page,
    /// Number of body columns and the current one (0-based).
    columns: u32,
    col: u32,
    /// Width of one column, the gap between columns, the left margin and the printable width (mm).
    col_w: f32,
    gap: f32,
    left: f32,
    print_w: f32,
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
        self.col = 0;
    }

    fn flow(&mut self, band: &Band, rows: &HashMap<String, usize>) {
        if self.body && self.y + band.height > self.limit {
            self.next_slot();
        }
        let xf = self.column_transform();
        place_with(self.page, band, self.y, rows, &mut self.items, xf);
        self.y += band.height;
        self.body = true;
    }

    /// Moves to the next column on this sheet, or to a new sheet after the last column.
    fn next_slot(&mut self) {
        if self.col + 1 < self.columns {
            self.col += 1;
            self.y = self.top + self.header_h;
        } else {
            self.new_sheet();
        }
    }

    /// Makes sure `need` mm of body space is free here, moving on first when it is not.
    fn keep(&mut self, need: f32) {
        if self.body && self.y + need > self.limit {
            self.next_slot();
        }
    }

    /// Where a body band goes in the current column: (scale, from x, to x), or None for one column.
    fn column_transform(&self) -> Option<(f32, f32, f32)> {
        if self.columns <= 1 {
            return None;
        }
        let scale = self.col_w / self.print_w;
        let to = self.left + self.col as f32 * (self.col_w + self.gap);
        Some((scale, self.left, to))
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

/// Prints a subreport: its pages are laid out in place, one block per page, starting at the
/// component's left edge and the flow position. Text is evaluated with the subreport's own data.
fn flow_subreport(flow: &mut Flow, comp: &Component, parent: &Report, depth: usize) {
    const MAX_DEPTH: usize = 4;
    let Some(path) = comp.subreport.as_deref() else { return };
    if comp.rect.y > flow.y {
        flow.y = comp.rect.y.min(flow.limit);
    }
    let sub = if depth < MAX_DEPTH { crate::model::subreport::load(path, parent.file_path.as_deref()) } else { None };
    let Some(sub) = sub else {
        let mut note = Component::default();
        note.comp_type = ComponentType::Text;
        note.name = "SubreportMissing".to_string();
        note.rect = RectMm::new(comp.rect.x, flow.y, comp.rect.w.max(60.0), 6.0);
        note.text = if path.trim().is_empty() {
            "Subreport: no file chosen".to_string()
        } else {
            format!("Subreport not found: {path}")
        };
        note.font_size = 8.0;
        note.text_color = [200, 0, 0, 255];
        note.word_wrap = false;
        flow.items.push(Item { comp: note, rows: HashMap::new() });
        flow.y += 6.0;
        flow.body = true;
        return;
    };

    let sheets = build_report_sheets(&sub, depth + 1);
    let total = sheets.len();
    for (k, sheet) in sheets.iter().enumerate() {
        let page = &sub.pages[sheet.page_idx];
        let (top, left) = (page.margins.top, page.margins.left);
        let mut vars = report_vars(&sub);
        vars.insert("PageNumber".to_string(), (k + 1).to_string());
        vars.insert("TotalPageCount".to_string(), total.to_string());

        // Each subreport page is one block: as tall as its lowest item.
        let block_h = sheet.items.iter().map(|i| i.comp.rect.y + i.comp.rect.h - top).fold(0.0_f32, f32::max);
        if k > 0 || (flow.body && flow.y + block_h > flow.limit) {
            flow.new_sheet();
        }
        let base_y = flow.y;
        for item in &sheet.items {
            let mut c = item.comp.clone();
            c.rect.x = comp.rect.x + (c.rect.x - left);
            c.rect.y = base_y + (c.rect.y - top);
            if matches!(c.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox) {
                let eval = EvalContext { dictionary: &sub.dictionary, row_index: &item.rows, vars: &vars };
                c.text = eval.evaluate(&c.text);
            }
            flow.items.push(Item { comp: c, rows: HashMap::new() });
        }
        flow.y = base_y + block_h.max(0.0);
        flow.body = true;
    }
}
/// A page item in flow order: a band, or a table.
enum Flowable<'a> {
    Band(&'a Band),
    Table(&'a Component),
    Sub(&'a Component),
}

/// What a table row is, for paging: a repeated header goes in front of body and footer rows
/// that start a new sheet.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TableRow {
    Header,
    Body,
    Footer,
}

/// Prints a table: the header, one body row per data row (or one body row), then the footer.
fn flow_table(flow: &mut Flow, comp: &Component, dict: &Dictionary) {
    let Some(spec) = comp.table.as_ref() else { return };
    let empty = HashMap::new();
    // Start where the table was placed, unless the flow is already further down the sheet.
    if comp.rect.y > flow.y {
        flow.y = comp.rect.y.min(flow.limit);
    }

    let header: Vec<String> = spec.columns.iter().map(|c| c.header.clone()).collect();
    place_table_row(flow, comp, spec, &header, &empty, TableRow::Header);

    let body: Vec<String> = spec.columns.iter().map(|c| c.body.clone()).collect();
    match spec.source.as_deref() {
        None => place_table_row(flow, comp, spec, &body, &empty, TableRow::Body),
        Some(name) => {
            if let Some(ds) = dict.data_sources.iter().find(|d| d.name == name) {
                for i in 0..ds.rows.len() {
                    let rows = HashMap::from([(ds.name.clone(), i)]);
                    place_table_row(flow, comp, spec, &body, &rows, TableRow::Body);
                }
            }
        }
    }

    if spec.columns.iter().any(|c| !c.footer.is_empty()) {
        let footer: Vec<String> = spec.columns.iter().map(|c| c.footer.clone()).collect();
        place_table_row(flow, comp, spec, &footer, &empty, TableRow::Footer);
    }
}

/// Places one table row at the flow position, moving to a new sheet first when it does not fit.
/// The header is repeated at the top of a new sheet, except before a header row.
fn place_table_row(
    flow: &mut Flow,
    comp: &Component,
    spec: &TableSpec,
    texts: &[String],
    rows: &HashMap<String, usize>,
    kind: TableRow,
) {
    let row_h = spec.row_height.max(3.0);
    if flow.body && flow.y + row_h > flow.limit {
        flow.new_sheet();
        if kind != TableRow::Header {
            let header: Vec<String> = spec.columns.iter().map(|c| c.header.clone()).collect();
            emit_table_row(flow, comp, spec, &header, &HashMap::new(), TableRow::Header);
        }
    }
    emit_table_row(flow, comp, spec, texts, rows, kind);
}

/// Adds the cells of one row at the current flow position and moves down by the row height.
fn emit_table_row(
    flow: &mut Flow,
    comp: &Component,
    spec: &TableSpec,
    texts: &[String],
    rows: &HashMap<String, usize>,
    kind: TableRow,
) {
    let row_h = spec.row_height.max(3.0);
    let header = kind == TableRow::Header;
    let mut x = comp.rect.x;
    for (i, col) in spec.columns.iter().enumerate() {
        let mut cell = Component::default();
        cell.comp_type = ComponentType::Text;
        cell.name = format!("{}_{}", comp.name, i);
        cell.rect = RectMm::new(x, flow.y, col.width, row_h);
        cell.text = texts.get(i).cloned().unwrap_or_default();
        cell.font_size = spec.font_size;
        cell.bold = header;
        cell.word_wrap = true;
        cell.align_h = if header { AlignH::Center } else { AlignH::Left };
        cell.align_v = AlignV::Center;
        cell.border_sides = BorderSides::all();
        cell.border_color = [160, 160, 160, 255];
        cell.border_width = 0.2;
        cell.text_color = comp.text_color;
        if header {
            cell.bg_color = [238, 241, 246, 255];
        }
        flow.items.push(Item { comp: cell, rows: rows.clone() });
        x += col.width;
    }
    flow.y += row_h;
    flow.body = true;
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
    place_with(page, band, y, rows, items, None);
}

/// Like place_into; with xf = (scale, from x, to x) the band is moved into a column and scaled to its width.
fn place_with(
    page: &Page,
    band: &Band,
    y: f32,
    rows: &HashMap<String, usize>,
    items: &mut Vec<Item>,
    xf: Option<(f32, f32, f32)>,
) {
    let dy = y - band.top;
    for c in page.components.iter().filter(|c| c.parent_band_id.as_deref() == Some(band.id.as_str())) {
        let mut comp = c.clone();
        comp.rect.y += dy;
        if let Some((scale, from, to)) = xf {
            comp.rect.x = to + (comp.rect.x - from) * scale;
            comp.rect.w *= scale;
        }
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

pub(super) fn page_size(app: &ReportDesignerApp, sheet: &Sheet) -> (f32, f32) {
    app.report.pages[sheet.page_idx].effective_dimensions()
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// The watermark text, centred on the sheet and rotated by its angle, behind the content.
fn draw_watermark(painter: &egui::Painter, wm: &Watermark, page_rect: Rect, zoom: f32) {
    let size = wm.font_size * rt::px_per_pt(zoom);
    let color = Color32::from_rgba_unmultiplied(wm.color[0], wm.color[1], wm.color[2], wm.color[3]);
    let galley = painter.layout_no_wrap(wm.text.clone(), egui::FontId::proportional(size), color);
    let (w, h) = (galley.size().x, galley.size().y);
    let a = wm.angle.to_radians();
    // The shape rotates around its top-left corner: move it so the text centre stays on the sheet centre.
    let half = Vec2::new(w / 2.0, h / 2.0);
    let rotated = Vec2::new(half.x * a.cos() - half.y * a.sin(), half.x * a.sin() + half.y * a.cos());
    let pos = page_rect.center() - rotated;
    let mut shape = egui::epaint::TextShape::new(pos, galley, color);
    shape.angle = a;
    painter.add(egui::Shape::Text(shape));
}
fn draw_paper(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect.translate(Vec2::new(0.0, 2.0)).expand(1.0), 2.0, Color32::from_black_alpha(28));
    painter.rect_filled(rect, 0.0, PAPER);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, RULE));
}

fn draw_component(
    painter: &egui::Painter,
    comp: &Component,
    page_rect: Rect,
    mm_px: f32,
    zoom: f32,
    images: &super::images::ImageCache,
) {
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
            let text_h = 11.0 * zoom;
            let bars_rect = Rect::from_min_max(bc_rect.min, Pos2::new(bc_rect.max.x, bc_rect.max.y - text_h));
            match crate::model::barcode::bars(&comp.barcode_format, &comp.text) {
                Ok(bars) => {
                    for (start, width) in bars {
                        let x0 = bars_rect.min.x + start * bars_rect.width();
                        let x1 = x0 + width * bars_rect.width();
                        painter.rect_filled(
                            Rect::from_min_max(Pos2::new(x0, bars_rect.min.y), Pos2::new(x1, bars_rect.max.y)),
                            0.0,
                            Color32::BLACK,
                        );
                    }
                }
                Err(e) => {
                    painter.text(
                        bars_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Barcode: {e}"),
                        egui::FontId::proportional(9.0 * zoom),
                        Color32::RED,
                    );
                }
            }
            painter.text(
                bc_rect.center_bottom() + Vec2::new(0.0, -1.0),
                egui::Align2::CENTER_BOTTOM,
                &comp.text,
                egui::FontId::monospace(9.0 * zoom),
                Color32::BLACK,
            );
        }        ComponentType::Line => {
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
        ComponentType::Image => {
            if let Some(texture) = comp.image_data.as_ref().and_then(|b| images.get(&super::images::key(b))) {
                super::images::paint(painter, texture, c_rect);
            }
        }
        // Tables are not rendered yet; they keep their place on the page.
        ComponentType::Table => {}
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

pub(super) fn base_vars(app: &ReportDesignerApp) -> HashMap<String, String> {
    report_vars(&app.report)
}

/// The values every page of report can use: its variables, the date and the time.
pub(super) fn report_vars(report: &Report) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for v in &report.dictionary.variables {
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

/// The text of one component as HTML: one block per paragraph (with its alignment and list
/// marker), styled spans for the runs. Without HTML tags the text is escaped as written.
fn text_html(comp: &Component, text: &str) -> String {
    let mut out = String::new();
    for para in parse_paragraphs(text, comp.allow_html_tags) {
        let align = match para.align.unwrap_or(comp.align_h) {
            AlignH::Center => "center",
            AlignH::Right => "right",
            _ => "left",
        };
        let mut spans = String::new();
        if let Some(bullet) = para.bullet {
            spans.push_str(&escape_html(&bullet.label()));
        }
        for run in &para.runs {
            let mut style = String::new();
            if run.bold || comp.bold {
                style.push_str("font-weight:bold;");
            }
            if run.italic || comp.italic {
                style.push_str("font-style:italic;");
            }
            let mut decoration = Vec::new();
            if run.underline || comp.underline {
                decoration.push("underline");
            }
            if run.strike {
                decoration.push("line-through");
            }
            if !decoration.is_empty() {
                style.push_str(&format!("text-decoration:{};", decoration.join(" ")));
            }
            if let Some(c) = run.color {
                style.push_str(&format!("color:{};", css_color(c)));
            }
            if let Some(bg) = run.highlight {
                style.push_str(&format!("background:{};", css_color(bg)));
            }
            if let Some(family) = &run.family {
                style.push_str(&format!("font-family:'{}';", family.replace('\'', "")));
            }
            let size = run.size_pt.unwrap_or(comp.font_size * run.size_scale);
            style.push_str(&format!("font-size:{size:.2}pt;"));
            spans.push_str(&format!("<span style=\"{style}\">{}</span>", escape_html(&run.text)));
        }
        if spans.is_empty() {
            spans.push_str("&nbsp;");
        }
        out.push_str(&format!("<div style=\"margin:0;text-align:{align};\">{spans}</div>"));
    }
    out
}
fn sheets_to_html(app: &ReportDesignerApp) -> String {
    let sheets = build_sheets(app);
    let total = sheets.len();
    let vars = base_vars(app);
    let running = RunningTotals::new(&app.report.dictionary);
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
            let mut item_vars = sheet_vars.clone();
            running.apply(&item.rows, &mut item_vars);
            let eval = EvalContext {
                dictionary: &app.report.dictionary,
                row_index: &item.rows,
                vars: &item_vars,
            };
            let text = if matches!(comp.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox) {
                eval.evaluate(&comp.text)
            } else {
                String::new()
            };

            let comp_c = comp.with_conditions(&text);
            let comp = &comp_c;
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
    fn groups_repeat_header_and_data_per_group() {
        let mut app = ReportDesignerApp::new(None, None);
        let page = &mut app.report.pages[0];
        page.bands.clear();
        page.components.clear();
        page.bands.push(Band {
            id: "gh".into(),
            name: "GroupHeader1".into(),
            band_type: BandType::GroupHeader,
            top: 10.0,
            height: 5.0,
            data_source: None,
            group_by: Some("detail.tgl".into()),
            keep_together: false,
            detail_of: None,
        });
        page.bands.push(Band {
            id: "db".into(),
            name: "DataBand1".into(),
            band_type: BandType::DataBand,
            top: 20.0,
            height: 5.0,
            data_source: Some("detail".into()),
            group_by: None,
            keep_together: false,
            detail_of: None,
        });
        let mut data_cell = Component::default();
        data_cell.parent_band_id = Some("db".into());
        data_cell.text = "{detail.nama}".into();
        data_cell.rect = RectMm::new(0.0, 20.0, 50.0, 5.0);
        page.components.push(data_cell);
        let mut header_cell = Component::default();
        header_cell.parent_band_id = Some("gh".into());
        header_cell.text = "{detail.tgl}".into();
        header_cell.rect = RectMm::new(0.0, 10.0, 50.0, 5.0);
        page.components.push(header_cell);

        let row = |tgl: &str, nama: &str| {
            let mut m = HashMap::new();
            m.insert("tgl".to_string(), tgl.to_string());
            m.insert("nama".to_string(), nama.to_string());
            m
        };
        app.report.dictionary.data_sources = vec![DataSource {
            name: "detail".into(),
            alias: "detail".into(),
            columns: vec![
                Column { name: "tgl".into(), data_type: "System.String".into() },
                Column { name: "nama".into(), data_type: "System.String".into() },
            ],
            rows: vec![row("A", "x"), row("A", "y"), row("B", "z")],
        }];

        let sheets = build_sheets(&app);
        let items: Vec<&Item> = sheets.iter().flat_map(|s| s.items.iter()).collect();
        let headers: Vec<usize> = items
            .iter()
            .filter(|i| i.comp.parent_band_id.as_deref() == Some("gh"))
            .map(|i| i.rows["detail"])
            .collect();
        let data = items.iter().filter(|i| i.comp.parent_band_id.as_deref() == Some("db")).count();
        assert_eq!(headers, vec![0, 2], "one header per group, starting at its first row");
        assert_eq!(data, 3, "every row prints once inside its group");
    }
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

#[cfg(test)]
mod pagination_tests {
    use super::*;
    use crate::model::{Band, BandType, Column, DataSource, Report};

    fn band(id: &str, t: BandType, top: f32, height: f32, source: Option<&str>, group: Option<&str>, keep: bool) -> Band {
        Band {
            id: id.into(),
            name: id.into(),
            band_type: t,
            top,
            height,
            data_source: source.map(|s| s.into()),
            group_by: group.map(|s| s.into()),
            keep_together: keep,
            detail_of: None,
        }
    }

    fn comp_in(band_id: &str, text: &str) -> Component {
        let mut c = Component::default();
        c.parent_band_id = Some(band_id.into());
        c.text = text.into();
        c.rect = RectMm::new(10.0, 0.0, 100.0, 8.0);
        c
    }

    fn source(rows: &[(&str, &str)]) -> DataSource {
        let col = |n: &str| Column { name: n.into(), data_type: "System.String".into() };
        DataSource {
            name: "d".into(),
            alias: "d".into(),
            columns: vec![col("g"), col("n")],
            rows: rows
                .iter()
                .map(|(g, n)| HashMap::from([("g".to_string(), g.to_string()), ("n".to_string(), n.to_string())]))
                .collect(),
        }
    }

    fn app_with(rows: &[(&str, &str)]) -> ReportDesignerApp {
        let mut app = ReportDesignerApp::new(None, None);
        app.report = Report::blank();
        app.report.dictionary.data_sources = vec![source(rows)];
        app
    }

    #[test]
    fn second_column_takes_rows_before_a_new_sheet() {
        let rows: Vec<(&str, &str)> = (0..40).map(|_| ("x", "1")).collect();
        let mut app = app_with(&rows);
        let page = &mut app.report.pages[0];
        page.columns = 2;
        page.bands = vec![band("db", BandType::DataBand, 0.0, 10.0, Some("d"), None, false)];
        page.components.push(comp_in("db", "{d.n}"));

        let sheets = build_sheets(&app);
        assert_eq!(sheets.len(), 1, "40 rows fit on one sheet in two columns");
        let page = &app.report.pages[0];
        let split = page.margins.left + page.printable_width() / 2.0;
        let second_column = sheets[0].items.iter().filter(|i| i.comp.rect.x > split).count();
        assert!(second_column > 0 && second_column < 40, "some rows are in the second column");
    }

    #[test]
    fn keep_together_moves_a_group_that_does_not_fit() {
        // Group A: 20 rows (210 mm). Group B: 8 rows (90 mm), more than the 67 mm left on the sheet.
        let mut rows: Vec<(&str, &str)> = (0..20).map(|_| ("A", "1")).collect();
        rows.extend((0..8).map(|_| ("B", "2")));

        let sheets_for = |keep: bool| {
            let mut app = app_with(&rows);
            let page = &mut app.report.pages[0];
            page.bands = vec![
                band("gh", BandType::GroupHeader, 0.0, 10.0, None, Some("d.g"), keep),
                band("db", BandType::DataBand, 10.0, 10.0, Some("d"), None, false),
            ];
            page.components.push(comp_in("gh", "{d.g}"));
            page.components.push(comp_in("db", "{d.n}"));
            build_sheets(&app)
        };

        // Keep together: group B starts on the second sheet, with its header first.
        let kept = sheets_for(true);
        assert_eq!(kept.len(), 2);
        let first_item = &kept[1].items[0];
        assert_eq!(first_item.comp.parent_band_id.as_deref(), Some("gh"));
        assert_eq!(first_item.rows["d"], 20);

        // Without it, group B starts on the first sheet (its header fits with its first row).
        let split = sheets_for(false);
        let first_items_b = split[0].items.iter().filter(|i| i.comp.parent_band_id.as_deref() == Some("gh")).count();
        assert_eq!(first_items_b, 2, "headers of both groups are on the first sheet");
    }
}
#[cfg(test)]
mod master_detail_tests {
    use super::*;
    use crate::model::{Band, BandType, DataSource, Relation, Report};

    fn row(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn source(name: &str, rows: Vec<HashMap<String, String>>) -> DataSource {
        DataSource { name: name.into(), alias: name.into(), columns: Vec::new(), rows }
    }

    fn band(id: &str, top: f32, detail_of: Option<&str>, data_source: &str) -> Band {
        Band {
            id: id.into(),
            name: id.into(),
            band_type: BandType::DataBand,
            top,
            height: 10.0,
            data_source: Some(data_source.into()),
            group_by: None,
            keep_together: false,
            detail_of: detail_of.map(|s| s.into()),
        }
    }

    fn line(band_id: &str, text: &str) -> Component {
        let mut c = Component::default();
        c.parent_band_id = Some(band_id.into());
        c.text = text.into();
        c
    }

    #[test]
    fn details_follow_their_parent_and_nest() {
        let mut app = ReportDesignerApp::new(None, None);
        app.report = Report::blank();
        app.report.dictionary.data_sources = vec![
            source("orders", vec![row(&[("id", "1")]), row(&[("id", "2")]), row(&[("id", "3")])]),
            source(
                "details",
                vec![
                    row(&[("id", "d1"), ("order_id", "1"), ("item", "a")]),
                    row(&[("id", "d2"), ("order_id", "1"), ("item", "b")]),
                    row(&[("id", "d3"), ("order_id", "2"), ("item", "c")]),
                ],
            ),
            source(
                "parts",
                vec![
                    row(&[("detail_id", "d1"), ("part", "p1")]),
                    row(&[("detail_id", "d1"), ("part", "p2")]),
                    row(&[("detail_id", "d3"), ("part", "p3")]),
                ],
            ),
        ];
        app.report.dictionary.relations = vec![
            Relation { name: "items".into(), parent: "orders".into(), parent_column: "id".into(), child: "details".into(), child_column: "order_id".into() },
            Relation { name: "parts".into(), parent: "details".into(), parent_column: "id".into(), child: "parts".into(), child_column: "detail_id".into() },
        ];
        let page = &mut app.report.pages[0];
        page.bands = vec![
            band("mh", 0.0, None, "orders"),
            band("dt", 10.0, Some("items"), "details"),
            band("sb", 20.0, Some("parts"), "parts"),
        ];
        page.components.push(line("mh", "{orders.id}"));
        page.components.push(line("dt", "{details.item}"));
        page.components.push(line("sb", "{parts.part}"));

        let sheets = build_sheets(&app);
        let items: Vec<&Item> = sheets.iter().flat_map(|s| s.items.iter()).collect();
        let seq: Vec<(String, Option<usize>, Option<usize>, Option<usize>)> = items
            .iter()
            .map(|i| {
                (
                    i.comp.parent_band_id.clone().unwrap_or_default(),
                    i.rows.get("orders").copied(),
                    i.rows.get("details").copied(),
                    i.rows.get("parts").copied(),
                )
            })
            .collect();
        let want = vec![
            ("mh", Some(0), None, None),
            ("dt", Some(0), Some(0), None),
            ("sb", Some(0), Some(0), Some(0)),
            ("sb", Some(0), Some(0), Some(1)),
            ("dt", Some(0), Some(1), None),
            ("mh", Some(1), None, None),
            ("dt", Some(1), Some(2), None),
            ("sb", Some(1), Some(2), Some(2)),
            ("mh", Some(2), None, None),
        ];
        let want: Vec<(String, Option<usize>, Option<usize>, Option<usize>)> =
            want.into_iter().map(|(b, o, d, p)| (b.to_string(), o, d, p)).collect();
        assert_eq!(seq, want);
    }
}
#[cfg(test)]
mod relation_file_tests {
    use crate::model::{Band, BandType, DataSource, MrtHandler, Relation, Report};

    #[test]
    fn relations_and_detail_bands_survive_save_and_load() {
        let mut report = Report::blank();
        report.dictionary.data_sources = vec![
            DataSource { name: "orders".into(), alias: "orders".into(), columns: Vec::new(), rows: Vec::new() },
            DataSource { name: "details".into(), alias: "details".into(), columns: Vec::new(), rows: Vec::new() },
        ];
        report.dictionary.relations = vec![Relation {
            name: "items".into(),
            parent: "orders".into(),
            parent_column: "id".into(),
            child: "details".into(),
            child_column: "order_id".into(),
        }];
        report.pages[0].bands = vec![Band {
            id: "dt".into(),
            name: "dt".into(),
            band_type: BandType::DataBand,
            top: 10.0,
            height: 8.0,
            data_source: Some("details".into()),
            group_by: None,
            keep_together: false,
            detail_of: Some("items".into()),
        }];

        let json = MrtHandler::to_stimulsoft_json(&report).expect("serialise");
        let text = serde_json::to_string(&json).expect("text");
        let loaded = MrtHandler::from_mrt_content(&text).expect("load");
        assert_eq!(loaded.dictionary.relations, report.dictionary.relations);
        assert_eq!(loaded.pages[0].bands[0].detail_of.as_deref(), Some("items"));
    }
}
#[cfg(test)]
mod crosstab_page_tests {
    use super::*;
    use crate::model::{Column, CrossAgg, CrossTab, DataSource, MrtHandler, Report};

    fn row(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn ct() -> CrossTab {
        CrossTab {
            source: "sales".into(),
            row_column: "region".into(),
            column_column: "month".into(),
            value_column: "amount".into(),
            aggregate: CrossAgg::Sum,
        }
    }

    fn report_with_crosstab() -> Report {
        let mut report = Report::blank();
        let col = |n: &str| Column { name: n.into(), data_type: "System.String".into() };
        report.dictionary.data_sources = vec![DataSource {
            name: "sales".into(),
            alias: "sales".into(),
            columns: vec![col("region"), col("month"), col("amount")],
            rows: vec![
                row(&[("region", "N"), ("month", "Jan"), ("amount", "10")]),
                row(&[("region", "S"), ("month", "Feb"), ("amount", "4")]),
            ],
        }];
        let mut comp = Component::default();
        comp.rect = RectMm::new(10.0, 40.0, 120.0, 30.0);
        comp.crosstab = Some(ct());
        report.pages[0].components.push(comp);
        report
    }

    #[test]
    fn page_crosstab_prints_as_cells() {
        let mut app = ReportDesignerApp::new(None, None);
        app.report = report_with_crosstab();
        let sheets = build_sheets(&app);
        let items: Vec<&Item> = sheets.iter().flat_map(|s| s.items.iter()).collect();
        // Rows N, S and a total row; columns Feb, Jan and a total column; the corner: 4 x 4 cells.
        assert_eq!(items.len(), 16);
        assert!(items.iter().any(|i| i.comp.text == "10,00"));
        assert!(items.iter().all(|i| i.comp.crosstab.is_none()), "cells are plain text");
    }

    #[test]
    fn crosstab_survives_save_and_load() {
        let report = report_with_crosstab();
        let json = MrtHandler::to_stimulsoft_json(&report).expect("serialise");
        let text = serde_json::to_string(&json).expect("text");
        let loaded = MrtHandler::from_mrt_content(&text).expect("load");
        assert_eq!(loaded.pages[0].components[0].crosstab, Some(ct()));
    }
}
#[cfg(test)]
mod table_tests {
    use super::*;
    use crate::model::{Column, DataSource, Report, TableColumn, TableSpec};

    #[test]
    fn table_rows_repeat_the_header_on_a_new_sheet() {
        let mut app = ReportDesignerApp::new(None, None);
        app.report = Report::blank();
        app.report.dictionary.data_sources = vec![DataSource {
            name: "items".into(),
            alias: "items".into(),
            columns: vec![Column { name: "name".into(), data_type: "System.String".into() }],
            rows: (1..=6).map(|i| HashMap::from([("name".to_string(), format!("r{i}"))])).collect(),
        }];
        // Tall rows: three body rows fit under the header on an A4 sheet.
        let spec = TableSpec {
            source: Some("items".into()),
            columns: vec![TableColumn { width: 60.0, header: "Name".into(), body: "{items.name}".into(), footer: String::new() }],
            row_height: 60.0,
            font_size: 9.0,
        };
        let comp = Component { name: "T".into(), rect: RectMm::new(10.0, 10.0, 60.0, 10.0), table: Some(spec), ..Component::default() };
        app.report.pages[0].components.push(comp);

        let sheets = build_sheets(&app);
        assert_eq!(sheets.len(), 2, "six rows of 60 mm need two sheets");
        assert_eq!(sheets[1].items[0].comp.text, "Name", "the header starts the second sheet");
        let body: Vec<&Item> = sheets.iter().flat_map(|s| s.items.iter()).filter(|i| i.rows.contains_key("items")).collect();
        assert_eq!(body.len(), 6, "every data row prints once");
        assert_eq!(body[5].rows["items"], 5);
    }

    #[test]
    fn table_survives_save_and_load() {
        let mut report = Report::blank();
        let spec = TableSpec::default_for(&report.dictionary);
        let comp = Component { name: "T".into(), comp_type: ComponentType::Table, table: Some(spec.clone()), ..Component::default() };
        report.pages[0].components.push(comp);
        let json = crate::model::MrtHandler::to_stimulsoft_json(&report).expect("serialise");
        let text = serde_json::to_string(&json).expect("text");
        let loaded = crate::model::MrtHandler::from_mrt_content(&text).expect("load");
        assert_eq!(loaded.pages[0].components[0].table, Some(spec));
    }
}
#[cfg(test)]
mod subreport_tests {
    use super::*;
    use crate::model::{Band, BandType, DataSource, MrtHandler, Report};

    fn row(v: &str) -> HashMap<String, String> {
        HashMap::from([("v".to_string(), v.to_string())])
    }

    #[test]
    fn subreport_prints_its_own_rows_inside_the_parent() {
        let dir = std::env::temp_dir().join(format!("nusa_sub_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let main_path = dir.join("main.mrt");
        let sub_path = dir.join("sub.mrt");

        // The subreport: one data band over its own source, with two rows.
        let mut sub = Report::blank();
        sub.dictionary.data_sources = vec![DataSource { name: "subds".into(), alias: "subds".into(), columns: Vec::new(), rows: vec![row("a"), row("b")] }];
        sub.pages[0].bands = vec![Band {
            id: "sb".into(),
            name: "sb".into(),
            band_type: BandType::DataBand,
            top: 0.0,
            height: 8.0,
            data_source: Some("subds".into()),
            group_by: None,
            keep_together: false,
            detail_of: None,
        }];
        let mut line = Component::default();
        line.parent_band_id = Some("sb".into());
        line.text = "Row {subds.v}".into();
        sub.pages[0].components.push(line);
        MrtHandler::save_to_file(&sub, &sub_path).expect("save subreport");

        // The main report: a subreport component at the top of its page.
        let mut main = Report::blank();
        main.file_path = Some(main_path.to_string_lossy().to_string());
        let mut comp = Component::default();
        comp.name = "Sub1".into();
        comp.rect = RectMm::new(10.0, 20.0, 100.0, 20.0);
        comp.subreport = Some("sub.mrt".into());
        main.pages[0].components.push(comp);

        let mut app = ReportDesignerApp::new(None, None);
        app.report = main;
        let sheets = build_sheets(&app);
        let texts: Vec<&str> = sheets.iter().flat_map(|s| s.items.iter()).map(|i| i.comp.text.as_str()).collect();
        assert_eq!(texts, vec!["Row a", "Row b"]);
        let ys: Vec<f32> = sheets.iter().flat_map(|s| s.items.iter()).map(|i| i.comp.rect.y).collect();
        assert!(ys[1] > ys[0], "the second row is printed below the first");

        std::fs::remove_dir_all(&dir).ok();
    }
}