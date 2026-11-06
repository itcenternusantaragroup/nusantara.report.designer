use eframe::egui::{self, Color32};

use crate::model::*;
use super::designer::{ActiveTool, ReportDesignerApp};
use super::icons::{self, Icon};
use super::theme::{self, ACCENT, MUTED, PANEL};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolboxTab {
    Components,
    Dictionary,
    Outline,
}

pub struct ToolboxPanel;

const TAB_ID: &str = "tb_tab";

impl ToolboxPanel {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::SidePanel::left("toolbox_panel")
            .resizable(true)
            .default_width(230.0)
            .min_width(190.0)
            .max_width(340.0)
            .frame(egui::Frame::side_top_panel(&ctx.style())
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(10.0, 10.0)))
            .show(ctx, |ui| {
                let id = egui::Id::new(TAB_ID);
                let mut current_tab = ctx
                    .data(|d| d.get_temp::<ToolboxTab>(id))
                    .unwrap_or(ToolboxTab::Components);

                // Segmented control: three equal cells, the active one filled with the accent tint.
                ui.columns(3, |cols| {
                    let tabs = [
                        (ToolboxTab::Components, "Tools"),
                        (ToolboxTab::Dictionary, "Data"),
                        (ToolboxTab::Outline, "Outline"),
                    ];
                    for (i, (tab, label)) in tabs.into_iter().enumerate() {
                        let active = current_tab == tab;
                        let mut text = egui::RichText::new(label)
                            .color(if active { ACCENT } else { MUTED });
                        if active {
                            text = text.strong();
                        }
                        let btn = egui::Button::new(text)
                            .fill(if active { theme::ACCENT_SOFT } else { Color32::TRANSPARENT })
                            .min_size(egui::vec2(cols[i].available_width(), 24.0));
                        if cols[i].add(btn).clicked() {
                            current_tab = tab;
                        }
                    }
                });
                ctx.data_mut(|d| d.insert_temp(id, current_tab));

                ui.add_space(8.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match current_tab {
                        ToolboxTab::Components => Self::show_tools(app, ui),
                        ToolboxTab::Dictionary => Self::show_data(app, ui),
                        ToolboxTab::Outline => Self::show_outline(app, ui),
                    }
                });
            });
    }

    fn show_tools(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        let is_pointer = app.active_tool == ActiveTool::Pointer;
        if icons::row(ui, Some(Icon::Select), "Select", is_pointer).clicked() {
            app.active_tool = ActiveTool::Pointer;
        }

        ui.add_space(12.0);
        ui.label(theme::heading("Components"));
        ui.add_space(2.0);

        let tools = [
            (ComponentType::Text, "Text", Icon::Text),
            (ComponentType::Barcode, "Barcode", Icon::Barcode),
            (ComponentType::Image, "Image", Icon::Image),
            (ComponentType::Shape, "Shape", Icon::Shape),
            (ComponentType::Line, "Line", Icon::Line),
            (ComponentType::CheckBox, "Checkbox", Icon::Checkbox),
        ];
        for (comp_type, label, icon) in tools {
            let is_active = app.active_tool == ActiveTool::InsertComponent(comp_type);
            if icons::row(ui, Some(icon), label, is_active).clicked() {
                app.active_tool = ActiveTool::InsertComponent(comp_type);
                app.status_message = format!("Click the page to place a {}.", label.to_lowercase());
            }
        }

        ui.add_space(12.0);
        ui.label(theme::heading("Bands"));
        ui.add_space(2.0);

        let bands = [
            (BandType::ReportTitle, "Report title"),
            (BandType::PageHeader, "Page header"),
            (BandType::Header, "Header"),
            (BandType::DataBand, "Data"),
            (BandType::Footer, "Footer"),
            (BandType::PageFooter, "Page footer"),
            (BandType::ReportSummary, "Report summary"),
        ];
        for (band_type, label) in bands {
            let is_active = app.active_tool == ActiveTool::InsertBand(band_type);
            if icons::row(ui, Some(Icon::Band), label, is_active).clicked() {
                app.active_tool = ActiveTool::InsertBand(band_type);
                app.status_message = format!("Click the page to add a {} band.", label.to_lowercase());
            }
        }
    }

    fn show_data(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        if ui.button("Load JSON data...").on_hover_text("Load a JSON file as a data source").clicked() {
            app.load_json_datasource_dialog();
        }

        ui.add_space(12.0);
        ui.label(theme::heading("Data sources"));

        let mut insert_field = None;

        if app.report.dictionary.data_sources.is_empty() {
            ui.label(theme::caption("No data sources yet. Load a JSON file to add fields."));
        }

        for ds in &app.report.dictionary.data_sources {
            egui::CollapsingHeader::new(format!("{} ({} rows)", ds.name, ds.rows.len()))
                .default_open(true)
                .show(ui, |ui| {
                    for col in &ds.columns {
                        ui.horizontal(|ui| {
                            ui.label(&col.name);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Add").on_hover_text("Place this field on the page").clicked() {
                                    insert_field = Some(format!("{{{}.{}}}", ds.name, col.name));
                                }
                            });
                        });
                    }
                });
        }

        if let Some(expr) = insert_field {
            app.push_undo();
            let mut comp = Component::default();
            comp.name = format!("Field_{}", app.current_page().components.len() + 1);
            comp.text = expr;
            comp.rect = RectMm::new(20.0, 50.0, 45.0, 8.0);
            comp.border_sides = BorderSides::all();
            let new_id = comp.id.clone();
            app.current_page_mut().components.push(comp);
            app.selected_component_id = Some(new_id);
            app.status_message = "Added data field to the page.".to_string();
            app.refresh_code_buffer();
        }

        ui.add_space(12.0);
        ui.label(theme::heading("Variables"));

        let mut insert_var = None;
        if app.report.dictionary.variables.is_empty() {
            ui.label(theme::caption("No variables defined."));
        }
        for var in &app.report.dictionary.variables {
            ui.horizontal(|ui| {
                ui.label(&var.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Add").clicked() {
                        insert_var = Some(var.name.clone());
                    }
                });
            });
        }

        if let Some(var_name) = insert_var {
            app.push_undo();
            let mut comp = Component::default();
            comp.name = format!("Var_{}", app.current_page().components.len() + 1);
            comp.text = format!("{{{var_name}}}");
            comp.rect = RectMm::new(20.0, 50.0, 50.0, 8.0);
            let new_id = comp.id.clone();
            app.current_page_mut().components.push(comp);
            app.selected_component_id = Some(new_id);
            app.status_message = "Added variable to the page.".to_string();
            app.refresh_code_buffer();
        }
    }

    fn show_outline(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        let page = app.current_page();
        let p_name = page.name.clone();
        let bands_info: Vec<(String, String, BandType)> = page.bands.iter()
            .map(|b| (b.id.clone(), b.name.clone(), b.band_type)).collect();
        let comps_info: Vec<(String, String, ComponentType, Option<String>)> = page.components.iter()
            .map(|c| (c.id.clone(), c.name.clone(), c.comp_type, c.parent_band_id.clone())).collect();

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
                                if ui.selectable_label(is_c_sel, format!("{} ({})", c_name, c_type.display_name())).clicked() {
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
                        if ui.selectable_label(is_c_sel, format!("{} ({})", c_name, c_type.display_name())).clicked() {
                            app.selected_component_id = Some(c_id.clone());
                            app.selected_band_id = None;
                        }
                    }
                }
            });
    }
}
