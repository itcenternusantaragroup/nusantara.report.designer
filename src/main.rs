#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod model;
pub mod ui;

use eframe::egui;
use ui::ReportDesignerApp;

fn main() -> eframe::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut screenshot_path = None;
    let mut open_path = None;
    let mut start_preview = false;
    let mut start_about = false;
    let mut start_splash = false;
    let mut preview_sheet: Option<usize> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => screenshot_path = args.next(),
            "--open" => open_path = args.next(),
            "--preview" => start_preview = true,
            "--about" => start_about = true,
            "--splash" => start_splash = true,
            "--preview-sheet" => {
                start_preview = true;
                preview_sheet = args.next().and_then(|n| n.parse().ok());
            }
            _ => {}
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 850.0])
            .with_title(format!("Nusantara Report Designer {} (build {}) - [Untitled]", env!("CARGO_PKG_VERSION"), env!("APP_BUILD")))
            .with_icon(ui::brand::window_icon().unwrap_or_else(ui::icons::app_icon)),
        ..Default::default()
    };

    eframe::run_native(
        "Nusantara Report Designer",
        native_options,
        Box::new(|cc| {
            ui::theme::apply(&cc.egui_ctx);
            let mut app = ReportDesignerApp::new(screenshot_path, open_path);
            app.show_about = start_about;
            app.splash_forced = start_splash;
            app.preview_sheet = preview_sheet;
            if start_preview {
                app.app_mode = ui::designer::AppMode::Preview;
            }
            Ok(Box::new(app))
        }),
    )
}
