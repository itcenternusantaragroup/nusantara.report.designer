use eframe::egui::{self, Vec2};
use crate::model::Orientation;
use super::about;
use super::designer::ReportDesignerApp;

pub struct NewReportDialog;

impl NewReportDialog {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        if app.show_new_report_dialog {
            egui::Window::new("New Report Wizard")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size(Vec2::new(520.0, 360.0))
                .show(ctx, |ui| {
                    ui.heading("Select a Report Template");
                    ui.label("Choose a starting template for your new report:");
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        // Card 1: Blank Portrait
                        ui.group(|ui| {
                            ui.set_width(230.0);
                            ui.heading("📄 Blank Report (Portrait)");
                            ui.label("Standard A4 Portrait page with margins. Start drawing from scratch.");
                            ui.add_space(8.0);
                            if ui.button("Create Blank Portrait").clicked() {
                                app.create_blank_report(false);
                                app.show_new_report_dialog = false;
                            }
                        });

                        // Card 2: Blank Landscape
                        ui.group(|ui| {
                            ui.set_width(230.0);
                            ui.heading("📄 Blank Report (Landscape)");
                            ui.label("Standard A4 Landscape page. Ideal for wide data tables.");
                            ui.add_space(8.0);
                            if ui.button("Create Blank Landscape").clicked() {
                                app.create_blank_report(true);
                                app.show_new_report_dialog = false;
                            }
                        });
                    });

                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        // Card 3: Surat Tugas / HRMS Document
                        ui.group(|ui| {
                            ui.set_width(230.0);
                            ui.heading("📑 Surat Tugas / Official Letter");
                            ui.label("HRMS letter template with header, employee fields, and dual signatures.");
                            ui.add_space(8.0);
                            if ui.button("Create Surat Tugas").clicked() {
                                app.create_template_report("letter");
                                app.show_new_report_dialog = false;
                            }
                        });

                        // Card 4: Standard Sales / Invoice
                        ui.group(|ui| {
                            ui.set_width(230.0);
                            ui.heading("📊 Sales / Invoice Report");
                            ui.label("Header band, multi-column data band, total summary and barcode.");
                            ui.add_space(8.0);
                            if ui.button("Create Sales Report").clicked() {
                                app.create_template_report("sales");
                                app.show_new_report_dialog = false;
                            }
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            app.show_new_report_dialog = false;
                        }
                    });
                });
        }
    }
}

pub struct PageSetupDialog;

impl PageSetupDialog {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        if app.show_page_setup {
            egui::Window::new("Page Setup")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let page = app.current_page_mut();

                    ui.heading("Paper Dimensions");
                    ui.horizontal(|ui| {
                        if ui.button("A4 (210 x 297 mm)").clicked() {
                            page.width = 210.0;
                            page.height = 297.0;
                        }
                        if ui.button("Letter (215.9 x 279.4 mm)").clicked() {
                            page.width = 215.9;
                            page.height = 279.4;
                        }
                        if ui.button("Legal (215.9 x 355.6 mm)").clicked() {
                            page.width = 215.9;
                            page.height = 355.6;
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Width (mm):");
                        ui.add(egui::DragValue::new(&mut page.width).speed(1.0).range(50.0..=1000.0));
                        ui.label("Height (mm):");
                        ui.add(egui::DragValue::new(&mut page.height).speed(1.0).range(50.0..=1000.0));
                    });

                    ui.add_space(8.0);
                    ui.heading("Orientation");
                    ui.horizontal(|ui| {
                        if ui.selectable_label(page.orientation == Orientation::Portrait, "Portrait").clicked() {
                            page.orientation = Orientation::Portrait;
                        }
                        if ui.selectable_label(page.orientation == Orientation::Landscape, "Landscape").clicked() {
                            page.orientation = Orientation::Landscape;
                        }
                    });

                    ui.add_space(8.0);
                    ui.heading("Columns");
                    ui.horizontal(|ui| {
                        ui.label("Columns:");
                        ui.add(egui::DragValue::new(&mut page.columns).speed(0.2).range(1..=6));
                        ui.label("Gap:");
                        ui.add(egui::DragValue::new(&mut page.column_gap).speed(0.5).range(0.0..=30.0).suffix(" mm"));
                    });
                    ui.label(egui::RichText::new("Body bands flow down one column, then into the next.").weak());

                    ui.add_space(8.0);
                    ui.heading("Watermark");
                    let mut enabled = page.watermark.is_some();
                    if ui.checkbox(&mut enabled, "Show a text watermark on every sheet").changed() {
                        page.watermark = if enabled { Some(crate::model::Watermark::default()) } else { None };
                    }
                    if let Some(wm) = page.watermark.as_mut() {
                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            ui.text_edit_singleline(&mut wm.text);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.add(egui::DragValue::new(&mut wm.font_size).speed(1.0).range(8.0..=300.0).suffix(" pt"));
                            ui.label("Angle:");
                            ui.add(egui::DragValue::new(&mut wm.angle).speed(1.0).range(-90.0..=90.0).suffix(" deg"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Colour (alpha = opacity):");
                            let mut c = egui::Color32::from_rgba_unmultiplied(wm.color[0], wm.color[1], wm.color[2], wm.color[3]);
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                wm.color = [c.r(), c.g(), c.b(), c.a()];
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.heading("Margins (mm)");
                    ui.columns(2, |cols| {
                        cols[0].horizontal(|ui| {
                            ui.label("Left:");
                            ui.add(egui::DragValue::new(&mut page.margins.left).speed(0.5).range(0.0..=100.0));
                        });
                        cols[1].horizontal(|ui| {
                            ui.label("Right:");
                            ui.add(egui::DragValue::new(&mut page.margins.right).speed(0.5).range(0.0..=100.0));
                        });
                    });
                    ui.columns(2, |cols| {
                        cols[0].horizontal(|ui| {
                            ui.label("Top:");
                            ui.add(egui::DragValue::new(&mut page.margins.top).speed(0.5).range(0.0..=100.0));
                        });
                        cols[1].horizontal(|ui| {
                            ui.label("Bottom:");
                            ui.add(egui::DragValue::new(&mut page.margins.bottom).speed(0.5).range(0.0..=100.0));
                        });
                    });

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Done").clicked() {
                            app.show_page_setup = false;
                            app.refresh_code_buffer();
                        }
                    });
                });
        }

        about::show(app, ctx);
    }
}
