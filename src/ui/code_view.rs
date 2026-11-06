use eframe::egui::{self, Color32};
use super::designer::{AppMode, ReportDesignerApp};

pub struct CodeView;

impl CodeView {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style())
                .fill(Color32::from_rgb(245, 246, 248)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("MRT JSON definition").strong().color(Color32::from_rgb(33, 90, 160)));
                    ui.separator();

                    if ui.button("⬅ Back to Designer").clicked() {
                        app.app_mode = AppMode::Design;
                    }

                    ui.separator();

                    if ui.button("🔄 Refresh Code from Designer").clicked() {
                        app.refresh_code_buffer();
                        app.status_message = "Refreshed MRT code buffer.".to_string();
                    }

                    if ui.button("✔ Apply Changes to Designer").clicked() {
                        match app.apply_code_buffer() {
                            Ok(_) => {
                                app.status_message = "Successfully applied MRT changes to designer!".to_string();
                            }
                            Err(e) => {
                                app.status_message = format!("Failed to apply MRT changes: {e}");
                            }
                        }
                    }

                    ui.separator();

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ctx.output_mut(|o| o.copied_text = app.code_buffer.clone());
                        app.status_message = "Copied MRT JSON to clipboard.".to_string();
                    }

                    if ui.button("💾 Save as .mrt...").clicked() {
                        app.save_as_file_dialog();
                    }
                });

                ui.separator();

                ui.label(egui::RichText::new("You can view or directly edit the .mrt JSON below. Click 'Apply Changes' to update the visual canvas.").italics().size(11.0).color(Color32::GRAY));

                ui.add_space(4.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let text_edit = egui::TextEdit::multiline(&mut app.code_buffer)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .lock_focus(true)
                        .desired_width(f32::INFINITY);

                    if ui.add(text_edit).changed() {
                        app.code_is_dirty = true;
                    }
                });
            });
    }
}
