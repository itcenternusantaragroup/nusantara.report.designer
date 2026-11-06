use eframe::egui::{self, Color32, Stroke};
use crate::model::*;
use super::designer::{ActiveTool, AppMode, ReportDesignerApp, RibbonTab};
use super::icons::{chip, Icon};
use super::theme::{self, ACCENT, CHROME, INK, MUTED};

pub struct RibbonBar;

/// A ribbon tab: text button with an accent underline when active.
fn ribbon_tab(ui: &mut egui::Ui, active: bool, label: &str) -> bool {
    let mut text = egui::RichText::new(label).color(if active { INK } else { MUTED });
    if active {
        text = text.strong();
    }
    let resp = ui.add(egui::Button::new(text).frame(false));
    if active {
        let r = resp.rect;
        ui.painter().line_segment(
            [egui::pos2(r.min.x + 4.0, r.max.y + 2.0), egui::pos2(r.max.x - 4.0, r.max.y + 2.0)],
            Stroke::new(2.0_f32, ACCENT),
        );
    }
    resp.clicked()
}

/// A labelled cluster of ribbon controls. The caption sits under the controls.
fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.horizontal(add);
        ui.label(theme::caption(title));
    });
    ui.separator();
}

impl RibbonBar {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::TopBottomPanel::top("ribbon_panel")
            .frame(egui::Frame::side_top_panel(&ctx.style())
                .fill(CHROME)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                // Row 1: app name, File menu, tabs, and mode switch.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Nusantara Report Designer").size(14.0).strong().color(INK));
                    ui.add_space(12.0);

                    ui.menu_button("File", |ui| {
                        if ui.button("New report...    Ctrl+N").clicked() {
                            app.show_new_report_dialog = true;
                            ui.close_menu();
                        }
                        if ui.button("Open...    Ctrl+O").clicked() {
                            app.open_file_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Save    Ctrl+S").clicked() {
                            app.save_file_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Save as...").clicked() {
                            app.save_as_file_dialog();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Load JSON data source...").clicked() {
                            app.load_json_datasource_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Page setup...").clicked() {
                            app.show_page_setup = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("About").clicked() {
                            app.show_about = true;
                            ui.close_menu();
                        }
                    });

                    ui.add_space(8.0);

                    let tabs = [
                        (RibbonTab::Home, "Home"),
                        (RibbonTab::Insert, "Insert"),
                        (RibbonTab::PageLayout, "Page layout"),
                        (RibbonTab::View, "View"),
                    ];
                    for (tab, label) in tabs {
                        if ribbon_tab(ui, app.active_ribbon_tab == tab, label) {
                            app.active_ribbon_tab = tab;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Mode switch, displayed right-to-left: Design, Preview, Code.
                        if chip(ui, Some(Icon::Design), "Design", true, app.app_mode == AppMode::Design).clicked() {
                            app.app_mode = AppMode::Design;
                        }
                        if chip(ui, Some(Icon::Preview), "Preview", true, app.app_mode == AppMode::Preview).clicked() {
                            app.app_mode = AppMode::Preview;
                        }
                        if chip(ui, Some(Icon::Code), "MRT code", true, app.app_mode == AppMode::CodeView).clicked()
                            && app.app_mode != AppMode::CodeView
                        {
                            app.refresh_code_buffer();
                            app.app_mode = AppMode::CodeView;
                        }

                        ui.add_space(12.0);

                        if chip(ui, Some(Icon::New), "New", true, false).on_hover_text("New report (Ctrl+N)").clicked() {
                            app.show_new_report_dialog = true;
                        }
                        if chip(ui, Some(Icon::Open), "Open", true, false).on_hover_text("Open .mrt (Ctrl+O)").clicked() {
                            app.open_file_dialog();
                        }
                        if chip(ui, Some(Icon::Save), "Save", true, false).on_hover_text("Save .mrt (Ctrl+S)").clicked() {
                            app.save_file_dialog();
                        }
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Row 2: contents of the active tab.
                ui.horizontal_wrapped(|ui| {
                    match app.active_ribbon_tab {
                        RibbonTab::Home => {
                            group(ui, "Undo", |ui| {
                                if chip(ui, Some(Icon::Undo), "Undo", !app.undo_stack.is_empty(), false).clicked() {
                                    app.undo();
                                }
                                if chip(ui, Some(Icon::Redo), "Redo", !app.redo_stack.is_empty(), false).clicked() {
                                    app.redo();
                                }
                            });

                            let has_sel = app.selected_component_id.is_some();
                            group(ui, "Clipboard", |ui| {
                                if chip(ui, Some(Icon::Copy), "Copy", has_sel, false).clicked() {
                                    app.copy_selected();
                                }
                                if chip(ui, Some(Icon::Paste), "Paste", app.clipboard_component.is_some(), false).clicked() {
                                    app.paste_selected();
                                }
                                if chip(ui, Some(Icon::Duplicate), "Duplicate", has_sel, false).clicked() {
                                    app.duplicate_selected();
                                }
                                if chip(ui, Some(Icon::Delete), "Delete", has_sel || app.selected_band_id.is_some(), false).clicked() {
                                    app.delete_selected();
                                }
                            });

                            if let Some(comp) = app.selected_component_mut() {
                                group(ui, "Font", |ui| {
                                    egui::ComboBox::from_id_salt("rb_font_family")
                                        .width(110.0)
                                        .selected_text(&comp.font_family)
                                        .show_ui(ui, |ui| {
                                            for f in ["Calibri", "Arial", "Segoe UI", "Times New Roman", "Courier New", "Verdana"] {
                                                if ui.selectable_label(comp.font_family == f, f).clicked() {
                                                    comp.font_family = f.to_string();
                                                }
                                            }
                                        });

                                    let mut sz = comp.font_size;
                                    if ui.add(egui::DragValue::new(&mut sz).speed(0.5).range(6.0..=72.0).suffix(" pt")).changed() {
                                        comp.font_size = sz;
                                    }

                                    if ui.selectable_label(comp.bold, egui::RichText::new("B").strong()).clicked() {
                                        comp.bold = !comp.bold;
                                    }
                                    if ui.selectable_label(comp.italic, egui::RichText::new("I").italics()).clicked() {
                                        comp.italic = !comp.italic;
                                    }
                                    if ui.selectable_label(comp.underline, egui::RichText::new("U").underline()).clicked() {
                                        comp.underline = !comp.underline;
                                    }
                                });

                                group(ui, "Align", |ui| {
                                    if ui.selectable_label(comp.align_h == AlignH::Left, "Left").clicked() {
                                        comp.align_h = AlignH::Left;
                                    }
                                    if ui.selectable_label(comp.align_h == AlignH::Center, "Center").clicked() {
                                        comp.align_h = AlignH::Center;
                                    }
                                    if ui.selectable_label(comp.align_h == AlignH::Right, "Right").clicked() {
                                        comp.align_h = AlignH::Right;
                                    }
                                });

                                group(ui, "Color", |ui| {
                                    let mut tc = Color32::from_rgba_premultiplied(
                                        comp.text_color[0], comp.text_color[1], comp.text_color[2], comp.text_color[3]
                                    );
                                    ui.label("Text");
                                    if ui.color_edit_button_srgba(&mut tc).changed() {
                                        comp.text_color = [tc.r(), tc.g(), tc.b(), tc.a()];
                                    }

                                    let mut bg = Color32::from_rgba_premultiplied(
                                        comp.bg_color[0], comp.bg_color[1], comp.bg_color[2], comp.bg_color[3]
                                    );
                                    ui.label("Fill");
                                    if ui.color_edit_button_srgba(&mut bg).changed() {
                                        comp.bg_color = [bg.r(), bg.g(), bg.b(), bg.a()];
                                    }
                                });

                                group(ui, "Border", |ui| {
                                    let has_borders = comp.border_sides.has_any();
                                    if ui.selectable_label(has_borders, "Outline").clicked() {
                                        comp.border_sides = if has_borders {
                                            BorderSides::none()
                                        } else {
                                            BorderSides::all()
                                        };
                                    }
                                });
                            } else {
                                ui.label(theme::caption("Select a component on the page to change its font, alignment and colors."));
                            }
                        }
                        RibbonTab::Insert => {
                            group(ui, "Components", |ui| {
                                let comps = [
                                    (ComponentType::Text, "Text", Icon::Text),
                                    (ComponentType::Barcode, "Barcode", Icon::Barcode),
                                    (ComponentType::Image, "Image", Icon::Image),
                                    (ComponentType::Shape, "Shape", Icon::Shape),
                                    (ComponentType::Line, "Line", Icon::Line),
                                    (ComponentType::CheckBox, "Checkbox", Icon::Checkbox),
                                ];
                                for (ct, lbl, ic) in comps {
                                    let is_active = app.active_tool == ActiveTool::InsertComponent(ct);
                                    if chip(ui, Some(ic), lbl, true, is_active).clicked() {
                                        app.active_tool = ActiveTool::InsertComponent(ct);
                                        app.status_message = format!("Click the page to place a {}.", lbl.to_lowercase());
                                    }
                                }
                            });

                            group(ui, "Bands", |ui| {
                                let bands = [
                                    (BandType::ReportTitle, "Title"),
                                    (BandType::Header, "Header"),
                                    (BandType::DataBand, "Data"),
                                    (BandType::Footer, "Footer"),
                                    (BandType::PageFooter, "Page footer"),
                                ];
                                for (bt, lbl) in bands {
                                    let is_active = app.active_tool == ActiveTool::InsertBand(bt);
                                    if chip(ui, Some(Icon::Band), lbl, true, is_active).clicked() {
                                        app.active_tool = ActiveTool::InsertBand(bt);
                                        app.status_message = format!("Click the page to add a {} band.", lbl.to_lowercase());
                                    }
                                }
                            });
                        }
                        RibbonTab::PageLayout => {
                            let page = app.current_page_mut();

                            group(ui, "Orientation", |ui| {
                                if chip(ui, Some(Icon::Portrait), "Portrait", true, page.orientation == Orientation::Portrait).clicked() {
                                    page.orientation = Orientation::Portrait;
                                }
                                if chip(ui, Some(Icon::Landscape), "Landscape", true, page.orientation == Orientation::Landscape).clicked() {
                                    page.orientation = Orientation::Landscape;
                                }
                            });

                            group(ui, "Paper size", |ui| {
                                if ui.button("A4").clicked() {
                                    page.width = 210.0;
                                    page.height = 297.0;
                                }
                                if ui.button("Letter").clicked() {
                                    page.width = 215.9;
                                    page.height = 279.4;
                                }
                                if ui.button("Legal").clicked() {
                                    page.width = 215.9;
                                    page.height = 355.6;
                                }
                            });

                            group(ui, "Margins", |ui| {
                                if ui.button("Normal 10 mm").clicked() {
                                    page.margins = Margins { left: 10.0, right: 10.0, top: 10.0, bottom: 10.0 };
                                }
                                if ui.button("Narrow 5 mm").clicked() {
                                    page.margins = Margins { left: 5.0, right: 5.0, top: 5.0, bottom: 5.0 };
                                }
                                if ui.button("Wide 20 mm").clicked() {
                                    page.margins = Margins { left: 20.0, right: 20.0, top: 20.0, bottom: 20.0 };
                                }
                            });

                            group(ui, "Setup", |ui| {
                                if ui.button("Page setup...").clicked() {
                                    app.show_page_setup = true;
                                }
                            });
                        }
                        RibbonTab::View => {
                            group(ui, "Show", |ui| {
                                ui.checkbox(&mut app.show_grid, "Grid");
                                ui.checkbox(&mut app.snap_to_grid, "Snap to grid");
                                ui.checkbox(&mut app.show_rulers, "Rulers");
                            });

                            group(ui, "Zoom", |ui| {
                                if chip(ui, Some(Icon::FitPage), "Fit page", true, app.fit_to_window)
                                    .on_hover_text("Zoom so the whole page is visible")
                                    .clicked()
                                {
                                    app.fit_to_window = true;
                                }
                                if ui.button("-").clicked() {
                                    app.zoom = (app.zoom - 0.15).max(0.3);
                                    app.fit_to_window = false;
                                }
                                if ui.button("+").clicked() {
                                    app.zoom = (app.zoom + 0.15).min(3.0);
                                    app.fit_to_window = false;
                                }
                                if ui.button("100%").clicked() {
                                    app.zoom = 1.0;
                                    app.fit_to_window = false;
                                }
                                if ui.button("150%").clicked() {
                                    app.zoom = 1.5;
                                    app.fit_to_window = false;
                                }
                            });
                        }
                    }
                });
            });
    }
}
