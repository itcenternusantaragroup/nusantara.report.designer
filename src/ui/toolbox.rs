use eframe::egui::{self, Color32, Frame, Margin, Rect, Sense, Stroke, Vec2};

use crate::model::*;
use super::designer::ReportDesignerApp;
use super::icons::{self, Icon};
use super::ribbon::add_page;
use super::theme::{self, ACCENT, PANEL, RULE};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolboxTab {
    Pages,
    Data,
    Outline,
}

pub struct ToolboxPanel;

const TAB_ID: &str = "tb_tab";

impl ToolboxPanel {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::SidePanel::left("toolbox_panel")
            .resizable(true)
            .default_width(220.0)
            .min_width(180.0)
            .max_width(320.0)
            .frame(Frame::side_top_panel(&ctx.style())
                .fill(PANEL)
                .inner_margin(Margin::symmetric(10.0, 8.0)))
            .show(ctx, |ui| {
                let id = egui::Id::new(TAB_ID);
                let mut current_tab = ctx
                    .data(|d| d.get_temp::<ToolboxTab>(id))
                    .unwrap_or(ToolboxTab::Pages);

                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (ToolboxTab::Pages, "Pages"),
                        (ToolboxTab::Data, "Data"),
                        (ToolboxTab::Outline, "Outline"),
                    ] {
                        if icons::text_tab(ui, current_tab == tab, label) {
                            current_tab = tab;
                        }
                    }
                });
                ctx.data_mut(|d| d.insert_temp(id, current_tab));

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                egui::ScrollArea::vertical().show(ui, |ui| match current_tab {
                    ToolboxTab::Pages => Self::show_pages(app, ui),
                    ToolboxTab::Data => super::dictionary_panel::show(app, ui),
                    ToolboxTab::Outline => Self::show_outline(app, ui),
                });
            });
    }

    /// Page thumbnails, like the slide filmstrip in Google Slides.
    fn show_pages(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        if icons::chip(ui, Some(Icon::Plus), "New page", true, false).clicked() {
            add_page(app);
        }
        ui.add_space(10.0);

        let width = (ui.available_width() - 16.0).clamp(120.0, 190.0);
        for i in 0..app.report.pages.len() {
            let selected = i == app.current_page_idx;
            let resp = thumbnail(ui, &app.report.pages[i], width, selected);
            ui.vertical_centered(|ui| {
                ui.label(theme::caption(format!("{}", i + 1)));
            });
            ui.add_space(8.0);
            if resp.clicked() {
                app.current_page_idx = i;
                app.selected_component_id = None;
                app.selected_band_id = None;
            }
        }
    }

    fn show_outline(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        let page = app.current_page();
        let p_name = page.name.clone();
        let bands_info: Vec<(String, String, BandType)> = page
            .bands
            .iter()
            .map(|b| (b.id.clone(), b.name.clone(), b.band_type))
            .collect();
        let comps_info: Vec<(String, String, ComponentType, Option<String>)> = page
            .components
            .iter()
            .map(|c| (c.id.clone(), c.name.clone(), c.comp_type, c.parent_band_id.clone()))
            .collect();

        egui::CollapsingHeader::new(p_name)
            .default_open(true)
            .show(ui, |ui| {
                for (b_id, b_name, b_type) in bands_info {
                    let is_b_sel = app.selected_band_id.as_deref() == Some(&b_id);
                    let label = format!("{} · {}", b_name, b_type.display_name());
                    if ui.selectable_label(is_b_sel, label).clicked() {
                        app.selected_band_id = Some(b_id.clone());
                        app.selected_component_id = None;
                    }

                    ui.indent(&b_id, |ui| {
                        for (c_id, c_name, c_type, parent_opt) in &comps_info {
                            if parent_opt.as_deref() == Some(&b_id) {
                                let is_c_sel = app.selected_component_id.as_deref() == Some(c_id);
                                if ui
                                    .selectable_label(is_c_sel, format!("{} ({})", c_name, c_type.display_name()))
                                    .clicked()
                                {
                                    app.selected_component_id = Some(c_id.clone());
                                    app.selected_band_id = None;
                                }
                            }
                        }
                    });
                }

                for (c_id, c_name, c_type, parent_opt) in &comps_info {
                    if parent_opt.is_none() {
                        let is_c_sel = app.selected_component_id.as_deref() == Some(c_id);
                        if ui
                            .selectable_label(is_c_sel, format!("{} ({})", c_name, c_type.display_name()))
                            .clicked()
                        {
                            app.selected_component_id = Some(c_id.clone());
                            app.selected_band_id = None;
                        }
                    }
                }
            });
    }
}

/// A small preview of one page: paper, band strips and components, scaled to `width`.
fn thumbnail(ui: &mut egui::Ui, page: &Page, width: f32, selected: bool) -> egui::Response {
    let (pw, ph) = page.effective_dimensions();
    let scale = width / pw;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, ph * scale), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect.translate(Vec2::new(0.0, 1.5)), 2.0, Color32::from_black_alpha(20));
        painter.rect_filled(rect, 2.0, theme::PAPER);

        let at = |x: f32, y: f32| rect.min + Vec2::new(x * scale, y * scale);
        for band in &page.bands {
            let c = band.band_type.default_color();
            let tint = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 70);
            let band_rect = Rect::from_min_max(
                at(page.margins.left, band.top),
                at(page.margins.left + page.printable_width(), band.top + band.height),
            );
            painter.rect_filled(band_rect, 0.0, tint);
        }
        for comp in &page.components {
            let r = Rect::from_min_max(at(comp.rect.x, comp.rect.y), at(comp.rect.right(), comp.rect.bottom()));
            painter.rect_filled(r, 0.0, Color32::from_rgb(0xA8, 0xAE, 0xB5));
        }

        let edge = if selected {
            Stroke::new(2.0_f32, ACCENT)
        } else {
            Stroke::new(1.0_f32, RULE)
        };
        painter.rect_stroke(rect, 2.0, edge);
    }
    resp
}
