//! Visual theme for the designer: a quiet graphite chrome, a warm-neutral desk
//! behind the page, white paper, and one ink-blue accent used only for state
//! (selection, active tab, active tool).

use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Vec2, Visuals};

/// Canvas background behind the page.
pub const DESK: Color32 = Color32::from_rgb(0xE4, 0xE7, 0xEC);
/// The page itself.
pub const PAPER: Color32 = Color32::WHITE;
/// Ribbon, status bar, rulers.
pub const CHROME: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF7);
/// Side panels.
pub const PANEL: Color32 = Color32::from_rgb(0xFA, 0xFB, 0xFC);
/// Hairlines and borders.
pub const RULE: Color32 = Color32::from_rgb(0xD6, 0xDB, 0xE2);
/// Primary text.
pub const INK: Color32 = Color32::from_rgb(0x1C, 0x21, 0x29);
/// Secondary text, captions, ticks.
pub const MUTED: Color32 = Color32::from_rgb(0x6A, 0x72, 0x7F);
/// The single accent colour.
pub const ACCENT: Color32 = Color32::from_rgb(0x2B, 0x59, 0xC3);
/// Accent tint for selected rows and segmented controls.
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE4, 0xEB, 0xF9);

/// Installs fonts and visuals. Call once when the app is created.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut v = Visuals::light();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0_f32,RULE);
    v.window_rounding = Rounding::same(6.0);
    v.menu_rounding = Rounding::same(6.0);
    v.extreme_bg_color = PAPER;
    v.faint_bg_color = CHROME;
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0_f32,ACCENT);
    v.hyperlink_color = ACCENT;

    let radius = Rounding::same(4.0);

    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.weak_bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32,RULE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32,INK);
    v.widgets.noninteractive.rounding = radius;

    v.widgets.inactive.bg_fill = PAPER;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32,RULE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32,INK);
    v.widgets.inactive.rounding = radius;

    v.widgets.hovered.bg_fill = CHROME;
    v.widgets.hovered.weak_bg_fill = CHROME;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32,MUTED);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32,INK);
    v.widgets.hovered.rounding = radius;

    v.widgets.active.bg_fill = ACCENT_SOFT;
    v.widgets.active.weak_bg_fill = ACCENT_SOFT;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32,ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32,INK);
    v.widgets.active.rounding = radius;

    v.widgets.open.bg_fill = CHROME;
    v.widgets.open.weak_bg_fill = CHROME;
    v.widgets.open.bg_stroke = Stroke::new(1.0_f32,RULE);
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32,INK);
    v.widgets.open.rounding = radius;

    ctx.style_mut(|s| {
        s.visuals = v;
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(8.0, 3.0);
        s.spacing.interact_size.y = 22.0;
        s.spacing.indent = 14.0;
    });
}

/// Uses Segoe UI from the Windows font folder when present, so the UI matches
/// the OS; otherwise keeps egui's bundled font.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let candidates = [r"C:\Windows\Fonts\segoeui.ttf"];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("system_ui".to_owned(), Arc::new(FontData::from_owned(bytes)));
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "system_ui".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

/// Small grey heading used above a group of controls.
pub fn caption(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).size(11.0).color(MUTED)
}

/// Section heading inside side panels. Sentence case, no tracking, no icons.
pub fn heading(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).size(12.0).strong().color(INK)
}
