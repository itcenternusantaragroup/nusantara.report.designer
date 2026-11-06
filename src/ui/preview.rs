use std::collections::HashMap;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use crate::model::*;
use super::designer::{AppMode, ReportDesignerApp};

pub struct PreviewView;

impl PreviewView {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style())
                .fill(Color32::from_rgb(220, 222, 225)))
            .show(ctx, |ui| {
                // Preview Toolbar
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("REPORT PREVIEW").strong().color(Color32::from_rgb(20, 80, 160)));
                    ui.separator();

                    if ui.button("⬅ Back to Designer").clicked() {
                        app.app_mode = AppMode::Design;
                    }

                    ui.separator();

                    if ui.button("📄 Export HTML...").clicked() {
                        Self::export_html_dialog(app);
                    }

                    if ui.button("📝 Export Text...").clicked() {
                        Self::export_text_dialog(app);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+").clicked() {
                            app.zoom = (app.zoom + 0.15).min(2.5);
                        }
                        if ui.button("-").clicked() {
                            app.zoom = (app.zoom - 0.15).max(0.5);
                        }
                        ui.label(format!("{:.0}%", app.zoom * 100.0));
                    });
                });

                ui.separator();

                // Scrollable rendered document
                egui::ScrollArea::both().show(ui, |ui| {
                    Self::render_preview_document(app, ui);
                });
            });
    }

    fn render_preview_document(app: &ReportDesignerApp, ui: &mut egui::Ui) {
        let zoom = app.zoom;
        let page = app.current_page();
        let (page_w_mm, page_h_mm) = page.effective_dimensions();

        let mm_to_px = 3.78 * zoom;
        let page_w_px = page_w_mm * mm_to_px;
        let page_h_px = page_h_mm * mm_to_px;

        let offset = Pos2::new(40.0, 20.0);
        let total_size = Vec2::new(page_w_px + offset.x * 2.0, page_h_px + offset.y * 2.0);
        let (_resp, painter) = ui.allocate_painter(total_size, egui::Sense::hover());

        let page_rect = Rect::from_min_size(offset, Vec2::new(page_w_px, page_h_px));

        // Draw Paper Drop Shadow & White Sheet
        painter.rect_filled(page_rect.translate(Vec2::new(4.0, 4.0)), 3.0, Color32::from_black_alpha(40));
        painter.rect_filled(page_rect, 0.0, Color32::WHITE);
        painter.rect_stroke(page_rect, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(190, 195, 200)));

        // Build variable dictionary
        let mut vars_map: HashMap<String, String> = HashMap::new();
        for v in &app.report.dictionary.variables {
            vars_map.insert(v.name.clone(), v.value.clone());
        }
        vars_map.insert("PageNumber".to_string(), "1".to_string());
        vars_map.insert("TotalPageCount".to_string(), "1".to_string());
        vars_map.insert("Date".to_string(), "2024-06-13".to_string());
        vars_map.insert("Time".to_string(), "15:00".to_string());

        // Find primary data source
        let default_ds = app.report.dictionary.data_sources.first();

        let title_band = page.bands.iter().find(|b| b.band_type == BandType::ReportTitle);
        let header_band = page.bands.iter().find(|b| b.band_type == BandType::Header);
        let data_band = page.bands.iter().find(|b| b.band_type == BandType::DataBand);
        let footer_band = page.bands.iter().find(|b| b.band_type == BandType::Footer);
        let page_footer_band = page.bands.iter().find(|b| b.band_type == BandType::PageFooter);

        let mut current_y_mm = page.margins.top;

        // 1. Render Report Title Band
        if let Some(band) = title_band {
            for comp in &page.components {
                if comp.parent_band_id.as_deref() == Some(&band.id) {
                    let evaluated_text = Self::evaluate_expr(&comp.text, &vars_map, None);
                    let mut rendered_comp = comp.clone();
                    rendered_comp.text = evaluated_text;
                    Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                }
            }
            current_y_mm = band.top + band.height + 2.0;
        }

        // 2. Render Header Band
        if let Some(band) = header_band {
            let dy = current_y_mm - band.top;
            for comp in &page.components {
                if comp.parent_band_id.as_deref() == Some(&band.id) {
                    let mut rendered_comp = comp.clone();
                    rendered_comp.rect.y += dy;
                    Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                }
            }
            current_y_mm += band.height;
        }

        // 3. Render Data Band
        if let Some(band) = data_band {
            let active_ds = band.data_source.as_ref()
                .and_then(|ds_name| app.report.dictionary.data_sources.iter().find(|d| &d.name == ds_name))
                .or(default_ds);

            let template_comps: Vec<&Component> = page.components.iter()
                .filter(|c| c.parent_band_id.as_deref() == Some(&band.id))
                .collect();

            if let Some(ds) = active_ds {
                if !ds.rows.is_empty() {
                    for row_data in &ds.rows {
                        for comp in &template_comps {
                            let evaluated_text = Self::evaluate_expr(&comp.text, &vars_map, Some(row_data));
                            let mut rendered_comp = (*comp).clone();
                            rendered_comp.rect.y = current_y_mm + (comp.rect.y - band.top);
                            rendered_comp.text = evaluated_text;
                            Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                        }
                        current_y_mm += band.height;
                    }
                } else {
                    for comp in &template_comps {
                        let evaluated_text = Self::evaluate_expr(&comp.text, &vars_map, None);
                        let mut rendered_comp = (*comp).clone();
                        rendered_comp.text = evaluated_text;
                        Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                    }
                    current_y_mm += band.height;
                }
            } else {
                for comp in &template_comps {
                    let evaluated_text = Self::evaluate_expr(&comp.text, &vars_map, None);
                    let mut rendered_comp = (*comp).clone();
                    rendered_comp.text = evaluated_text;
                    Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                }
                current_y_mm += band.height;
            }
        }

        // 4. Render Footer Band
        if let Some(band) = footer_band {
            let dy = current_y_mm - band.top;
            for comp in &page.components {
                if comp.parent_band_id.as_deref() == Some(&band.id) {
                    let mut rendered_comp = comp.clone();
                    rendered_comp.rect.y += dy;
                    Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                }
            }
        }

        // 5. Render Page Footer Band
        if let Some(band) = page_footer_band {
            for comp in &page.components {
                if comp.parent_band_id.as_deref() == Some(&band.id) {
                    let evaluated_text = Self::evaluate_expr(&comp.text, &vars_map, None);
                    let mut rendered_comp = comp.clone();
                    rendered_comp.text = evaluated_text;
                    Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
                }
            }
        }

        // 6. Standalone components (not in any band)
        for comp in &page.components {
            if comp.parent_band_id.is_none() {
                let evaluated_text = Self::evaluate_expr(&comp.text, &vars_map, None);
                let mut rendered_comp = comp.clone();
                rendered_comp.text = evaluated_text;
                Self::draw_rendered_component(&painter, &rendered_comp, page_rect, mm_to_px, zoom);
            }
        }
    }

    fn evaluate_expr(expr: &str, vars: &HashMap<String, String>, row: Option<&HashMap<String, String>>) -> String {
        let mut result = expr.to_string();

        if let Some(row_map) = row {
            for (col_name, val) in row_map {
                result = result.replace(&format!("{{{col_name}}}"), val);
                // Also with table prefix like {Orders.col_name} or {employee.col_name}
                for prefix in ["Orders", "employee", "detail", "root", "ijin_karyawan"] {
                    result = result.replace(&format!("{{{prefix}.{col_name}}}"), val);
                }
            }
        }

        for (var_name, val) in vars {
            result = result.replace(&format!("{{{var_name}}}"), val);
        }

        result
    }

    fn draw_rendered_component(
        painter: &egui::Painter,
        comp: &Component,
        page_rect: Rect,
        mm_to_px: f32,
        zoom: f32,
    ) {
        let c_rect = Rect::from_min_size(
            Pos2::new(page_rect.min.x + comp.rect.x * mm_to_px, page_rect.min.y + comp.rect.y * mm_to_px),
            Vec2::new(comp.rect.w * mm_to_px, comp.rect.h * mm_to_px),
        );

        if comp.bg_color[3] > 0 {
            let fill_c = Color32::from_rgba_premultiplied(comp.bg_color[0], comp.bg_color[1], comp.bg_color[2], comp.bg_color[3]);
            painter.rect_filled(c_rect, 0.0, fill_c);
        }

        if comp.border_sides.has_any() {
            let border_c = Color32::from_rgba_premultiplied(comp.border_color[0], comp.border_color[1], comp.border_color[2], comp.border_color[3]);
            let stroke = Stroke::new((comp.border_width * mm_to_px).max(1.0), border_c);

            if comp.border_sides.top { painter.line_segment([c_rect.left_top(), c_rect.right_top()], stroke); }
            if comp.border_sides.bottom { painter.line_segment([c_rect.left_bottom(), c_rect.right_bottom()], stroke); }
            if comp.border_sides.left { painter.line_segment([c_rect.left_top(), c_rect.left_bottom()], stroke); }
            if comp.border_sides.right { painter.line_segment([c_rect.right_top(), c_rect.right_bottom()], stroke); }
        }

        match comp.comp_type {
            ComponentType::Text => {
                let tc = Color32::from_rgba_premultiplied(comp.text_color[0], comp.text_color[1], comp.text_color[2], comp.text_color[3]);
                let font_size = (comp.font_size * 1.33 * zoom).max(8.0);
                let font_id = egui::FontId::proportional(font_size);

                let align2 = match (comp.align_h, comp.align_v) {
                    (AlignH::Left, AlignV::Top) => egui::Align2::LEFT_TOP,
                    (AlignH::Left, AlignV::Center) => egui::Align2::LEFT_CENTER,
                    (AlignH::Left, AlignV::Bottom) => egui::Align2::LEFT_BOTTOM,
                    (AlignH::Center, AlignV::Top) => egui::Align2::CENTER_TOP,
                    (AlignH::Center, AlignV::Center) => egui::Align2::CENTER_CENTER,
                    (AlignH::Center, AlignV::Bottom) => egui::Align2::CENTER_BOTTOM,
                    (AlignH::Right, AlignV::Top) => egui::Align2::RIGHT_TOP,
                    (AlignH::Right, AlignV::Center) => egui::Align2::RIGHT_CENTER,
                    (AlignH::Right, AlignV::Bottom) => egui::Align2::RIGHT_BOTTOM,
                    _ => egui::Align2::LEFT_CENTER,
                };

                let pos = match align2 {
                    egui::Align2::LEFT_TOP => c_rect.left_top() + Vec2::new(3.0, 2.0),
                    egui::Align2::LEFT_CENTER => c_rect.left_center() + Vec2::new(3.0, 0.0),
                    egui::Align2::LEFT_BOTTOM => c_rect.left_bottom() + Vec2::new(3.0, -2.0),
                    egui::Align2::CENTER_TOP => c_rect.center_top() + Vec2::new(0.0, 2.0),
                    egui::Align2::CENTER_CENTER => c_rect.center(),
                    egui::Align2::CENTER_BOTTOM => c_rect.center_bottom() + Vec2::new(0.0, -2.0),
                    egui::Align2::RIGHT_TOP => c_rect.right_top() + Vec2::new(-3.0, 2.0),
                    egui::Align2::RIGHT_CENTER => c_rect.right_center() + Vec2::new(-3.0, 0.0),
                    egui::Align2::RIGHT_BOTTOM => c_rect.right_bottom() + Vec2::new(-3.0, -2.0),
                };

                painter.with_clip_rect(c_rect).text(pos, align2, &comp.text, font_id, tc);
            }
            ComponentType::Barcode => {
                let bc_rect = c_rect.shrink(2.0);
                let mut bx = bc_rect.min.x;
                let bar_w = 2.0 * zoom;
                let mut toggle = false;
                while bx < bc_rect.max.x {
                    if toggle {
                        painter.rect_filled(
                            Rect::from_min_max(Pos2::new(bx, bc_rect.min.y), Pos2::new(bx + bar_w, bc_rect.max.y - 11.0)),
                            0.0,
                            Color32::BLACK,
                        );
                    }
                    bx += bar_w + (if (bx as usize) % 3 == 0 { 1.5 * bar_w } else { 0.5 * bar_w });
                    toggle = !toggle;
                }
                painter.text(bc_rect.center_bottom() + Vec2::new(0.0, -1.0), egui::Align2::CENTER_BOTTOM, &comp.text, egui::FontId::monospace(9.0 * zoom), Color32::BLACK);
            }
            _ => {}
        }
    }

    fn export_html_dialog(app: &mut ReportDesignerApp) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML Document (*.html)", &["html"])
            .set_file_name("report.html")
            .save_file()
        {
            let html_content = Self::generate_html(app);
            if let Err(e) = std::fs::write(&path, html_content) {
                app.status_message = format!("Export failed: {e}");
            } else {
                app.status_message = format!("Exported HTML report to {}", path.display());
            }
        }
    }

    fn export_text_dialog(app: &mut ReportDesignerApp) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text File (*.txt)", &["txt"])
            .set_file_name("report.txt")
            .save_file()
        {
            let text_content = Self::generate_text(app);
            if let Err(e) = std::fs::write(&path, text_content) {
                app.status_message = format!("Export failed: {e}");
            } else {
                app.status_message = format!("Exported Text report to {}", path.display());
            }
        }
    }

    fn generate_html(app: &ReportDesignerApp) -> String {
        let mut html = String::from("<!DOCTYPE html><html><head><meta charset='utf-8'><title>Report</title><style>");
        html.push_str("body{font-family:Arial,sans-serif;margin:40px;background:#f5f5f5;}");
        html.push_str(".page{background:white;padding:30px;max-width:800px;margin:0 auto;box-shadow:0 2px 8px rgba(0,0,0,0.1);}");
        html.push_str("table{width:100%;border-collapse:collapse;margin:20px 0;}");
        html.push_str("th{background:#2980b9;color:white;padding:8px;text-align:left;}");
        html.push_str("td{padding:8px;border-bottom:1px solid #ddd;}");
        html.push_str("</style></head><body><div class='page'>");
        html.push_str(&format!("<h1>{}</h1>", app.report.name));

        if let Some(ds) = app.report.dictionary.data_sources.first() {
            html.push_str("<table><thead><tr>");
            for col in &ds.columns {
                html.push_str(&format!("<th>{}</th>", col.name));
            }
            html.push_str("</tr></thead><tbody>");

            let empty = String::new();
            for row in &ds.rows {
                html.push_str("<tr>");
                for col in &ds.columns {
                    html.push_str(&format!("<td>{}</td>", row.get(&col.name).unwrap_or(&empty)));
                }
                html.push_str("</tr>");
            }
            html.push_str("</tbody></table>");
        }

        html.push_str("</div></body></html>");
        html
    }

    fn generate_text(app: &ReportDesignerApp) -> String {
        let mut txt = String::new();
        txt.push_str(&format!("========================================================================================\n"));
        txt.push_str(&format!("                                   {}\n", app.report.name.to_uppercase()));
        txt.push_str(&format!("========================================================================================\n"));

        if let Some(ds) = app.report.dictionary.data_sources.first() {
            for col in &ds.columns {
                txt.push_str(&format!("{:<15} ", col.name));
            }
            txt.push_str("\n----------------------------------------------------------------------------------------\n");

            let empty = String::new();
            for row in &ds.rows {
                for col in &ds.columns {
                    let val = row.get(&col.name).unwrap_or(&empty);
                    txt.push_str(&format!("{:<15} ", val));
                }
                txt.push_str("\n");
            }
        }
        txt.push_str("========================================================================================\n");
        txt
    }
}
