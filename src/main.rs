pub mod model;
pub mod ui;

use eframe::egui;
use ui::ReportDesignerApp;

fn main() -> eframe::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut screenshot_path = None;
    let mut open_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => screenshot_path = args.next(),
            "--open" => open_path = args.next(),
            _ => {}
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 850.0])
            .with_title("Nusantara Report Designer - [Untitled]")
            .with_icon(ui::icons::app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Nusantara Report Designer",
        native_options,
        Box::new(|cc| {
            ui::theme::apply(&cc.egui_ctx);
            Ok(Box::new(ReportDesignerApp::new(screenshot_path, open_path)))
        }),
    )
}
