//! The About window: logo, version and build, what the app does, and what it is built with.

use eframe::egui::{self, Align2, Frame, Margin, RichText, Vec2};

use super::designer::ReportDesignerApp;
use super::icons;
use super::theme::{self, CHROME, INK, MUTED, PAPER, RULE};

const WIDTH: f32 = 460.0;

pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
    if !app.show_about {
        return;
    }

    egui::Window::new("About Nusantara Report Designer")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .auto_sized()
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(Frame::none().fill(PAPER).rounding(10.0).stroke(egui::Stroke::new(1.0_f32, RULE)))
        .show(ctx, |ui| {
            ui.set_width(WIDTH);

            // Header: the logo on a light band.
            Frame::none()
                .fill(CHROME)
                .inner_margin(Margin::symmetric(24.0, 22.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.vertical_centered(|ui| match &app.brand {
                        Some(brand) => {
                            let size = brand.logo.size_vec2();
                            let w = 300.0;
                            ui.add(egui::Image::new(&brand.logo).fit_to_exact_size(Vec2::new(w, w * size.y / size.x)));
                        }
                        None => {
                            ui.label(RichText::new("Nusantara Report Designer").size(18.0).strong().color(INK));
                        }
                    });
                });

            Frame::none()
                .inner_margin(Margin::symmetric(28.0, 18.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                                .size(15.0)
                                .strong()
                                .color(INK),
                        );
                        ui.label(RichText::new(format!("Build {}", env!("APP_BUILD"))).size(12.0).color(MUTED));
                    });

                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(
                            "A desktop designer for .mrt report templates. Lay out bands and fields, \
                             preview with live data, and export to HTML or text.",
                        )
                        .color(INK),
                    );

                    ui.add_space(14.0);
                    section(ui, "Features", &[
                        "Bands, text, barcodes, lines, shapes and checkboxes",
                        "Rich text with HTML tags: bold, italic, colour, size and line breaks",
                        "Live preview with JSON data sources, with pagination and page header and footer",
                        "Open .mrt files in XML and JSON; save as .mrt (JSON)",
                        "Units in centimetres, inches and millimetres",
                    ]);

                    ui.add_space(12.0);
                    section(ui, "Built with", &["Rust", "egui 0.30 and eframe"]);

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("© 2026. All rights reserved.").size(11.0).color(MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::primary_button(ui, "Close").clicked() {
                                app.show_about = false;
                            }
                        });
                    });
                });
        });
}

fn section(ui: &mut egui::Ui, title: &str, items: &[&str]) {
    ui.label(theme::heading(title));
    ui.add_space(2.0);
    for item in items {
        ui.horizontal_top(|ui| {
            ui.label(RichText::new("•").color(MUTED));
            // Wrap inside the remaining width instead of stretching the window.
            ui.add(egui::Label::new(RichText::new(*item).color(INK)).wrap());
        });
    }
}
