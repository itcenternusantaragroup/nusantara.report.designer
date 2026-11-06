use eframe::egui::{self, Color32, Frame, Margin, Sense};

use crate::model::*;
use super::arrange::Align;
use super::designer::{ActiveTool, AppMode, ReportDesignerApp};
use super::icons::{self, Icon};
use super::theme::{self, PAPER, TOOLBAR};

const FONTS: [&str; 6] = ["Calibri", "Arial", "Segoe UI", "Times New Roman", "Courier New", "Verdana"];

/// The three top bars, styled after Google Slides: title bar, menu bar and toolbar.
pub struct RibbonBar;

impl RibbonBar {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        Self::title_bar(app, ctx);
        Self::menu_bar(app, ctx);
        Self::toolbar(app, ctx);
    }

    fn title_bar(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::TopBottomPanel::top("title_bar")
            .frame(Frame::none().fill(PAPER).inner_margin(Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    match &app.brand {
                        Some(brand) => {
                            ui.add(egui::Image::new(&brand.icon).fit_to_exact_size(egui::vec2(36.0, 36.0)));
                        }
                        None => {
                            let (logo_rect, _) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), Sense::hover());
                            icons::logo(ui.painter(), logo_rect);
                        }
                    }
                    ui.add_space(4.0);

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let title = app
                            .report
                            .file_path
                            .as_deref()
                            .and_then(|p| std::path::Path::new(p).file_stem())
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Untitled report".to_string());
                        ui.label(egui::RichText::new(title).size(18.0).color(theme::INK));
                        ui.label(egui::RichText::new("Nusantara Report Designer").size(11.0).color(theme::MUTED));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::primary_button(ui, "Save").on_hover_text("Save .mrt (Ctrl+S)").clicked() {
                            app.save_file_dialog();
                        }
                        ui.add_space(12.0);

                        if icons::text_tab(ui, app.app_mode == AppMode::CodeView, "MRT code")
                            && app.app_mode != AppMode::CodeView
                        {
                            app.refresh_code_buffer();
                            app.app_mode = AppMode::CodeView;
                        }
                        if icons::text_tab(ui, app.app_mode == AppMode::Preview, "Preview") {
                            app.app_mode = AppMode::Preview;
                        }
                        if icons::text_tab(ui, app.app_mode == AppMode::Design, "Design") {
                            app.app_mode = AppMode::Design;
                        }
                    });
                });
            });
    }

    fn menu_bar(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar")
            .frame(Frame::none().fill(PAPER).inner_margin(Margin::symmetric(8.0, 0.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    icons::menu(ui, "File", |ui| {
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
                    });

                    icons::menu(ui, "Edit", |ui| {
                        if ui.add_enabled(!app.undo_stack.is_empty(), egui::Button::new("Undo    Ctrl+Z")).clicked() {
                            app.undo();
                            ui.close_menu();
                        }
                        if ui.add_enabled(!app.redo_stack.is_empty(), egui::Button::new("Redo    Ctrl+Y")).clicked() {
                            app.redo();
                            ui.close_menu();
                        }
                        ui.separator();
                        let has_sel = app.selected_component_id.is_some();
                        if ui.add_enabled(has_sel, egui::Button::new("Copy    Ctrl+C")).clicked() {
                            app.copy_selected();
                            ui.close_menu();
                        }
                        if ui.add_enabled(app.clipboard_component.is_some(), egui::Button::new("Paste    Ctrl+V")).clicked() {
                            app.paste_selected();
                            ui.close_menu();
                        }
                        if ui.add_enabled(has_sel, egui::Button::new("Duplicate    Ctrl+D")).clicked() {
                            app.duplicate_selected();
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(has_sel || app.selected_band_id.is_some(), egui::Button::new("Delete    Del"))
                            .clicked()
                        {
                            app.delete_selected();
                            ui.close_menu();
                        }
                    });

                    icons::menu(ui, "View", |ui| {
                        ui.checkbox(&mut app.show_grid, "Grid");
                        ui.checkbox(&mut app.snap_to_grid, "Snap to grid");
                        ui.checkbox(&mut app.show_rulers, "Rulers");
                        ui.separator();
                        if ui.button("Fit page").clicked() {
                            app.fit_to_window = true;
                            ui.close_menu();
                        }
                        if ui.button("Zoom in").clicked() {
                            app.zoom = (app.zoom + 0.15).min(3.0);
                            app.fit_to_window = false;
                            ui.close_menu();
                        }
                        if ui.button("Zoom out").clicked() {
                            app.zoom = (app.zoom - 0.15).max(0.3);
                            app.fit_to_window = false;
                            ui.close_menu();
                        }
                        if ui.button("Actual size (100%)").clicked() {
                            app.zoom = 1.0;
                            app.fit_to_window = false;
                            ui.close_menu();
                        }
                    });

                    icons::menu(ui, "Insert", |ui| {
                        ui.label(theme::caption("Components"));
                        for (ct, lbl) in [
                            (ComponentType::Text, "Text"),
                            (ComponentType::Barcode, "Barcode"),
                            (ComponentType::Image, "Image"),
                            (ComponentType::Shape, "Shape"),
                            (ComponentType::Line, "Line"),
                            (ComponentType::CheckBox, "Checkbox"),
                        ] {
                            let tool = ActiveTool::InsertComponent(ct);
                            if ui.selectable_label(app.active_tool == tool, lbl).clicked() {
                                app.status_message = format!("Click the page to place a {}.", lbl.to_lowercase());
                                app.active_tool = tool;
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label(theme::caption("Bands"));
                        Self::band_items(app, ui);
                        ui.separator();
                        if ui.button("Table").on_hover_text("Table with a header, body rows from a data source and a footer").clicked() {
                            app.push_undo();
                            let spec = crate::model::TableSpec::default_for(&app.report.dictionary);
                            let mut c = Component::default();
                            c.name = format!("Table_{}", app.current_page().components.len() + 1);
                            c.comp_type = ComponentType::Table;
                            c.rect = RectMm::new(20.0, 60.0, spec.width(), spec.row_height * 2.0);
                            c.table = Some(spec);
                            let id = c.id.clone();
                            app.current_page_mut().components.push(c);
                            app.selected_component_id = Some(id);
                            app.selected_band_id = None;
                            app.refresh_code_buffer();
                            ui.close_menu();
                        }
                        if ui.button("Subreport").on_hover_text("Print another report (.mrt file) here, with its own data").clicked() {
                            app.push_undo();
                            let mut c = Component::default();
                            c.name = format!("Subreport_{}", app.current_page().components.len() + 1);
                            c.rect = RectMm::new(20.0, 60.0, 150.0, 30.0);
                            c.text = "[Subreport]".into();
                            c.subreport = Some(String::new());
                            let id = c.id.clone();
                            app.current_page_mut().components.push(c);
                            app.selected_component_id = Some(id);
                            app.selected_band_id = None;
                            app.refresh_code_buffer();
                            ui.close_menu();
                        }
                        if ui.button("Cross-tab").on_hover_text("Pivot table built from a data source").clicked() {
                            match crate::model::crosstab::default_for(&app.report.dictionary) {
                                Some(ct) => {
                                    app.push_undo();
                                    let mut c = Component::default();
                                    c.name = format!("CrossTab_{}", app.current_page().components.len() + 1);
                                    c.rect = RectMm::new(20.0, 60.0, 150.0, 60.0);
                                    c.text = "[Cross-tab]".into();
                                    c.crosstab = Some(ct);
                                    let id = c.id.clone();
                                    app.current_page_mut().components.push(c);
                                    app.selected_component_id = Some(id);
                                    app.selected_band_id = None;
                                    app.refresh_code_buffer();
                                }
                                None => app.status_message = "Load a data source first (Data tab).".to_string(),
                            }
                            ui.close_menu();
                        }
                        if ui.button("New page").clicked() {
                            add_page(app);
                            ui.close_menu();
                        }
                    });

                    icons::menu(ui, "Arrange", |ui| {
                        let n = app.selected_ids().len();
                        let two = n >= 2;
                        let three = n >= 3;
                        ui.label(theme::caption("Align (2+ selected)"));
                        for (al, lbl) in [
                            (Align::Left, "Align left"),
                            (Align::HCenter, "Align horizontal centers"),
                            (Align::Right, "Align right"),
                            (Align::Top, "Align top"),
                            (Align::VCenter, "Align vertical centers"),
                            (Align::Bottom, "Align bottom"),
                        ] {
                            if ui.add_enabled(two, egui::Button::new(lbl)).clicked() {
                                app.align_selected(al);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label(theme::caption("Distribute (3+ selected)"));
                        if ui.add_enabled(three, egui::Button::new("Distribute horizontally")).clicked() {
                            app.distribute_selected(true);
                            ui.close_menu();
                        }
                        if ui.add_enabled(three, egui::Button::new("Distribute vertically")).clicked() {
                            app.distribute_selected(false);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.add_enabled(n >= 1, egui::Button::new("Bring to front")).clicked() {
                            app.z_order(true);
                            ui.close_menu();
                        }
                        if ui.add_enabled(n >= 1, egui::Button::new("Send to back")).clicked() {
                            app.z_order(false);
                            ui.close_menu();
                        }
                    });

                    icons::menu(ui, "Page", |ui| {
                        let page = app.current_page_mut();
                        ui.label(theme::caption("Orientation"));
                        if ui.selectable_label(page.orientation == Orientation::Portrait, "Portrait").clicked() {
                            page.orientation = Orientation::Portrait;
                        }
                        if ui.selectable_label(page.orientation == Orientation::Landscape, "Landscape").clicked() {
                            page.orientation = Orientation::Landscape;
                        }
                        ui.separator();
                        ui.label(theme::caption("Paper size"));
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
                        ui.separator();
                        ui.label(theme::caption("Margins"));
                        if ui.button("Normal 10 mm").clicked() {
                            page.margins = Margins { left: 10.0, right: 10.0, top: 10.0, bottom: 10.0 };
                        }
                        if ui.button("Narrow 5 mm").clicked() {
                            page.margins = Margins { left: 5.0, right: 5.0, top: 5.0, bottom: 5.0 };
                        }
                        if ui.button("Wide 20 mm").clicked() {
                            page.margins = Margins { left: 20.0, right: 20.0, top: 20.0, bottom: 20.0 };
                        }
                        ui.separator();
                        if ui.button("Page setup...").clicked() {
                            app.show_page_setup = true;
                            ui.close_menu();
                        }
                    });

                    icons::menu(ui, "Help", |ui| {
                        if ui.button("Check for updates").clicked() {
                            app.start_update_check(true);
                            ui.close_menu();
                        }
                        if ui.button("About").clicked() {
                            app.show_about = true;
                            ui.close_menu();
                        }
                    });
                });
            });
    }

    fn band_items(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        for (bt, lbl) in [
            (BandType::ReportTitle, "Title"),
            (BandType::Header, "Header"),
            (BandType::DataBand, "Data"),
            (BandType::Footer, "Footer"),
            (BandType::PageFooter, "Page footer"),
            (BandType::GroupHeader, "Group header"),
            (BandType::GroupFooter, "Group footer"),
        ] {
            let tool = ActiveTool::InsertBand(bt);
            if ui.selectable_label(app.active_tool == tool, lbl).clicked() {
                app.status_message = format!("Click the page to add a {} band.", lbl.to_lowercase());
                app.active_tool = tool;
                ui.close_menu();
            }
        }
    }

    fn toolbar(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(Frame::none().fill(TOOLBAR).inner_margin(Margin::symmetric(10.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    if icons::button(ui, Icon::Undo, "Undo (Ctrl+Z)", !app.undo_stack.is_empty(), false).clicked() {
                        app.undo();
                    }
                    if icons::button(ui, Icon::Redo, "Redo (Ctrl+Y)", !app.redo_stack.is_empty(), false).clicked() {
                        app.redo();
                    }

                    ui.separator();

                    let zoom_label = if app.fit_to_window {
                        "Fit page".to_string()
                    } else {
                        format!("{:.0}%", app.zoom * 100.0)
                    };
                    egui::ComboBox::from_id_salt("zoom_combo")
                        .width(96.0)
                        .selected_text(zoom_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(app.fit_to_window, "Fit page").clicked() {
                                app.fit_to_window = true;
                            }
                            for pct in [50.0_f32, 75.0, 100.0, 125.0, 150.0, 200.0] {
                                let sel = !app.fit_to_window && (app.zoom * 100.0 - pct).abs() < 0.5;
                                if ui.selectable_label(sel, format!("{pct:.0}%")).clicked() {
                                    app.zoom = pct / 100.0;
                                    app.fit_to_window = false;
                                }
                            }
                        });

                    ui.separator();

                    let tools = [
                        (ActiveTool::Pointer, Icon::Select, "Select"),
                        (ActiveTool::InsertComponent(ComponentType::Text), Icon::Text, "Text box"),
                        (ActiveTool::InsertComponent(ComponentType::Barcode), Icon::Barcode, "Barcode"),
                        (ActiveTool::InsertComponent(ComponentType::Image), Icon::Image, "Image"),
                        (ActiveTool::InsertComponent(ComponentType::Shape), Icon::Shape, "Shape"),
                        (ActiveTool::InsertComponent(ComponentType::Line), Icon::Line, "Line"),
                        (ActiveTool::InsertComponent(ComponentType::CheckBox), Icon::Checkbox, "Checkbox"),
                    ];
                    for (tool, icon, tip) in tools {
                        let is_active = app.active_tool == tool;
                        if icons::button(ui, icon, tip, true, is_active).clicked() {
                            app.status_message = if tool == ActiveTool::Pointer {
                                "Select components on the page.".to_string()
                            } else {
                                format!("Click the page to place a {}.", tip.to_lowercase())
                            };
                            app.active_tool = tool;
                        }
                    }

                    icons::menu(ui, "Band", |ui| Self::band_items(app, ui));

                    ui.separator();

                    let has_sel = app.selected_component_id.is_some();
                    if icons::button(ui, Icon::Copy, "Copy (Ctrl+C)", has_sel, false).clicked() {
                        app.copy_selected();
                    }
                    if icons::button(ui, Icon::Paste, "Paste (Ctrl+V)", app.clipboard_component.is_some(), false).clicked() {
                        app.paste_selected();
                    }
                    if icons::button(ui, Icon::Delete, "Delete (Del)", has_sel || app.selected_band_id.is_some(), false).clicked() {
                        app.delete_selected();
                    }

                    ui.separator();

                    // Formatting applies to the selected component; the controls are greyed out otherwise.
                    format_controls(ui, app.selected_component_mut());
                });
            });
    }
}

/// Font, size, style, alignment, colours and border for one component.
/// With no selection (`comp` is None) every control is shown but disabled.
fn format_controls(ui: &mut egui::Ui, mut comp: Option<&mut Component>) {
    let enabled = comp.is_some();
    let (font, size, bold, italic, underline, align, text_c, fill_c, bordered) = match comp.as_deref() {
        Some(c) => (
            c.font_family.clone(),
            c.font_size,
            c.bold,
            c.italic,
            c.underline,
            c.align_h,
            c.text_color,
            c.bg_color,
            c.border_sides.has_any(),
        ),
        None => ("Arial".to_string(), 10.0, false, false, false, AlignH::Left, [0, 0, 0, 255], [0, 0, 0, 0], false),
    };

    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt("tb_font")
            .width(128.0)
            .selected_text(font.clone())
            .show_ui(ui, |ui| {
                for f in FONTS {
                    if ui.selectable_label(font == f, f).clicked() {
                        if let Some(c) = comp.as_deref_mut() {
                            c.font_family = f.to_string();
                        }
                    }
                }
            });

        let mut sz = size;
        if ui
            .add(egui::DragValue::new(&mut sz).speed(0.5).range(6.0..=72.0).suffix(" pt"))
            .changed()
        {
            if let Some(c) = comp.as_deref_mut() {
                c.font_size = sz;
            }
        }

        if icons::letter_button(ui, "B", "Bold", enabled, bold, false).clicked() {
            if let Some(c) = comp.as_deref_mut() {
                c.bold = !c.bold;
            }
        }
        if icons::letter_button(ui, "I", "Italic", enabled, italic, false).clicked() {
            if let Some(c) = comp.as_deref_mut() {
                c.italic = !c.italic;
            }
        }
        if icons::letter_button(ui, "U", "Underline", enabled, underline, true).clicked() {
            if let Some(c) = comp.as_deref_mut() {
                c.underline = !c.underline;
            }
        }

        ui.separator();

        for (al, icon, tip) in [
            (AlignH::Left, Icon::AlignLeft, "Align left"),
            (AlignH::Center, Icon::AlignCenter, "Align centre"),
            (AlignH::Right, Icon::AlignRight, "Align right"),
        ] {
            if icons::button(ui, icon, tip, enabled, align == al).clicked() {
                if let Some(c) = comp.as_deref_mut() {
                    c.align_h = al;
                }
            }
        }

        ui.separator();

        let mut tc = Color32::from_rgba_premultiplied(text_c[0], text_c[1], text_c[2], text_c[3]);
        if ui.color_edit_button_srgba(&mut tc).on_hover_text("Text colour").changed() {
            if let Some(c) = comp.as_deref_mut() {
                c.text_color = [tc.r(), tc.g(), tc.b(), tc.a()];
            }
        }
        let mut bg = Color32::from_rgba_premultiplied(fill_c[0], fill_c[1], fill_c[2], fill_c[3]);
        if ui.color_edit_button_srgba(&mut bg).on_hover_text("Fill colour").changed() {
            if let Some(c) = comp.as_deref_mut() {
                c.bg_color = [bg.r(), bg.g(), bg.b(), bg.a()];
            }
        }

        if icons::button(ui, Icon::Outline, "Border", enabled, bordered).clicked() {
            if let Some(c) = comp.as_deref_mut() {
                c.border_sides = if bordered { BorderSides::none() } else { BorderSides::all() };
            }
        }
    });
}

/// Adds an empty page after the last one and switches to it.
pub fn add_page(app: &mut ReportDesignerApp) {
    app.push_undo();
    app.report.pages.push(Page::default());
    app.current_page_idx = app.report.pages.len() - 1;
    app.selected_component_id = None;
    app.selected_band_id = None;
    app.refresh_code_buffer();
}
