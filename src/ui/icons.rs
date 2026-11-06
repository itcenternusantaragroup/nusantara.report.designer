//! Vector icons drawn with the egui painter, so they render the same on every
//! machine (no font glyphs needed). Each icon is designed on a 16 x 16 grid
//! and scaled to whatever rect it is painted into.

use std::f32::consts::PI;

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2};

use super::theme::{self, ACCENT, ACCENT_SOFT, CHROME, INK, MUTED, PAPER, RULE};

/// Text and icon colour for disabled controls.
pub const DISABLED: Color32 = Color32::from_rgb(0xB4, 0xBA, 0xC3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    New,
    Open,
    Save,
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    Delete,
    Select,
    Text,
    Barcode,
    Image,
    Shape,
    Line,
    Checkbox,
    Band,
    FitPage,
    Portrait,
    Landscape,
    Design,
    Preview,
    Code,
    AlignLeft,
    AlignCenter,
    AlignRight,
    ChevronDown,
    Plus,
    TextColor,
    FillColor,
    Outline,
}

/// Paints `icon` inside `rect` using `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let k = rect.width() / 16.0;
    let stroke = Stroke::new((1.4 * k).max(1.0), color);
    let m = |x: f32, y: f32| Pos2::new(rect.min.x + x * k, rect.min.y + y * k);

    let line = |pts: &[(f32, f32)], closed: bool| {
        let mut v: Vec<Pos2> = pts.iter().map(|&(x, y)| m(x, y)).collect();
        if closed {
            if let Some(&first) = v.first() {
                v.push(first);
            }
        }
        painter.add(Shape::line(v, stroke));
    };
    let rr = |x0: f32, y0: f32, x1: f32, y1: f32| {
        line(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], true);
    };
    let filled = |x0: f32, y0: f32, x1: f32, y1: f32, c: Color32| {
        painter.rect_filled(Rect::from_min_max(m(x0, y0), m(x1, y1)), 0.0, c);
    };
    // A curved path with an arrow head at its last point.
    let arrow = |pts: Vec<(f32, f32)>| {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| m(x, y)).collect();
        let n = v.len();
        painter.add(Shape::line(v.clone(), stroke));
        let tip = v[n - 1];
        let back = -(tip - v[n - 2]).normalized();
        for deg in [40.0_f32, -40.0_f32] {
            let a = deg.to_radians();
            let dir = Vec2::new(back.x * a.cos() - back.y * a.sin(), back.x * a.sin() + back.y * a.cos());
            painter.line_segment([tip, tip + dir * (4.0 * k)], stroke);
        }
    };

    match icon {
        Icon::New => {
            line(&[(3.0, 1.0), (10.0, 1.0), (13.0, 4.0), (13.0, 15.0), (3.0, 15.0)], true);
            line(&[(10.0, 1.0), (10.0, 4.0), (13.0, 4.0)], false);
        }
        Icon::Open => {
            line(&[(1.5, 4.0), (6.0, 4.0), (7.5, 5.5), (14.5, 5.5), (14.5, 13.0), (1.5, 13.0)], true);
        }
        Icon::Save => {
            rr(2.0, 2.0, 14.0, 14.0);
            rr(5.0, 2.0, 11.0, 6.0);
            line(&[(5.0, 14.0), (5.0, 10.0), (11.0, 10.0), (11.0, 14.0)], false);
        }
        Icon::Undo => arrow(arc(8.0, 8.0, 4.5, 30.0, -160.0, 12)),
        Icon::Redo => arrow(arc(8.0, 8.0, 4.5, 150.0, 340.0, 12)),
        Icon::Copy => {
            rr(5.0, 1.0, 14.0, 11.0);
            paper_fill(painter, Rect::from_min_max(m(2.0, 5.0), m(11.0, 15.0)));
            rr(2.0, 5.0, 11.0, 15.0);
        }
        Icon::Paste => {
            rr(3.0, 2.5, 13.0, 15.0);
            filled(5.5, 1.0, 10.5, 4.0, color);
        }
        Icon::Duplicate => {
            rr(1.5, 1.5, 10.0, 10.0);
            paper_fill(painter, Rect::from_min_max(m(6.0, 6.0), m(14.5, 14.5)));
            rr(6.0, 6.0, 14.5, 14.5);
        }
        Icon::Delete => {
            line(&[(2.0, 4.0), (14.0, 4.0)], false);
            line(&[(6.0, 4.0), (6.0, 2.0), (10.0, 2.0), (10.0, 4.0)], false);
            line(&[(3.5, 4.0), (4.5, 14.5), (11.5, 14.5), (12.5, 4.0)], false);
        }
        Icon::Select => {
            line(&[(4.0, 2.0), (4.0, 13.0), (6.8, 10.5), (8.6, 14.2), (10.4, 13.4), (8.6, 9.8), (12.2, 9.8)], true);
        }
        Icon::Text => {
            line(&[(3.0, 3.0), (13.0, 3.0)], false);
            line(&[(8.0, 3.0), (8.0, 13.5)], false);
            line(&[(5.5, 13.5), (10.5, 13.5)], false);
        }
        Icon::Barcode => {
            for (i, x) in [2.5_f32, 4.0, 5.5, 7.0, 8.5, 10.0, 11.5, 13.0].into_iter().enumerate() {
                if i % 3 != 2 {
                    line(&[(x, 3.0), (x, 13.0)], false);
                }
            }
        }
        Icon::Image => {
            rr(2.0, 3.0, 14.0, 13.0);
            line(&[(3.5, 11.5), (6.5, 8.0), (8.5, 10.0), (10.5, 7.5), (12.5, 11.5)], false);
            painter.circle_stroke(m(11.0, 6.0), 1.0 * k, stroke);
        }
        Icon::Shape => {
            painter.circle_stroke(m(8.0, 8.0), 5.5 * k, stroke);
        }
        Icon::Line => {
            line(&[(2.5, 13.5), (13.5, 2.5)], false);
        }
        Icon::Checkbox => {
            rr(2.5, 2.5, 13.5, 13.5);
            line(&[(5.0, 8.5), (7.2, 11.0), (11.0, 5.5)], false);
        }
        Icon::Band => {
            filled(1.5, 4.0, 14.5, 6.5, color);
            rr(1.5, 4.0, 14.5, 12.0);
        }
        Icon::FitPage => {
            line(&[(2.0, 5.0), (2.0, 2.0), (5.0, 2.0)], false);
            line(&[(11.0, 2.0), (14.0, 2.0), (14.0, 5.0)], false);
            line(&[(14.0, 11.0), (14.0, 14.0), (11.0, 14.0)], false);
            line(&[(5.0, 14.0), (2.0, 14.0), (2.0, 11.0)], false);
        }
        Icon::Portrait => {
            rr(4.0, 1.5, 12.0, 14.5);
        }
        Icon::Landscape => {
            rr(1.5, 4.0, 14.5, 12.0);
        }
        Icon::Design => {
            line(&[(3.0, 13.0), (4.0, 9.5), (11.0, 2.5), (13.5, 5.0), (6.5, 12.0), (3.0, 13.0)], false);
            line(&[(9.5, 4.0), (12.0, 6.5)], false);
        }
        Icon::Preview => {
            let mut up = Vec::new();
            let mut down = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let x = 1.5 + 13.0 * t;
                up.push((x, 8.0 - 4.5 * (PI * t).sin()));
                down.push((x, 8.0 + 4.5 * (PI * t).sin()));
            }
            line(&up, false);
            line(&down, false);
            painter.circle_stroke(m(8.0, 8.0), 2.0 * k, stroke);
        }
        Icon::AlignLeft => {
            line(&[(2.0, 4.0), (14.0, 4.0)], false);
            line(&[(2.0, 7.5), (10.0, 7.5)], false);
            line(&[(2.0, 11.0), (14.0, 11.0)], false);
            line(&[(2.0, 14.5), (8.0, 14.5)], false);
        }
        Icon::AlignCenter => {
            line(&[(2.0, 4.0), (14.0, 4.0)], false);
            line(&[(4.0, 7.5), (12.0, 7.5)], false);
            line(&[(2.0, 11.0), (14.0, 11.0)], false);
            line(&[(5.0, 14.5), (11.0, 14.5)], false);
        }
        Icon::AlignRight => {
            line(&[(2.0, 4.0), (14.0, 4.0)], false);
            line(&[(6.0, 7.5), (14.0, 7.5)], false);
            line(&[(2.0, 11.0), (14.0, 11.0)], false);
            line(&[(8.0, 14.5), (14.0, 14.5)], false);
        }
        Icon::ChevronDown => {
            line(&[(4.0, 6.0), (8.0, 10.0), (12.0, 6.0)], false);
        }
        Icon::Plus => {
            line(&[(8.0, 3.0), (8.0, 13.0)], false);
            line(&[(3.0, 8.0), (13.0, 8.0)], false);
        }
        Icon::TextColor => {
            line(&[(3.0, 12.0), (8.0, 2.5), (13.0, 12.0)], false);
            line(&[(5.0, 8.5), (11.0, 8.5)], false);
            filled(2.0, 13.8, 14.0, 15.5, color);
        }
        Icon::FillColor => {
            line(&[(4.0, 7.0), (8.5, 2.5), (13.0, 7.0), (8.5, 11.5)], true);
            filled(2.0, 13.8, 14.0, 15.5, color);
        }
        Icon::Outline => {
            rr(2.5, 2.5, 13.5, 13.5);
        }
        Icon::Code => {
            line(&[(5.0, 4.0), (1.5, 8.0), (5.0, 12.0)], false);
            line(&[(11.0, 4.0), (14.5, 8.0), (11.0, 12.0)], false);
            line(&[(9.5, 3.0), (6.5, 13.0)], false);
        }
    }
}

/// Points on a circle from `a0` to `a1` (degrees, y down), `n` segments.
fn arc(cx: f32, cy: f32, r: f32, a0: f32, a1: f32, n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
            (cx + r * t.cos(), cy + r * t.sin())
        })
        .collect()
}

fn paper_fill(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 0.0, PAPER);
}

/// A bordered button with an optional icon and a text label.
/// `selected` gives it the accent tint (used for toggles like the active tool).
pub fn chip(ui: &mut egui::Ui, icon: Option<Icon>, text: &str, enabled: bool, selected: bool) -> egui::Response {
    let color = if enabled { INK } else { DISABLED };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), egui::FontId::proportional(13.0), color);
    let icon_w = if icon.is_some() { 16.0 + 6.0 } else { 0.0 };
    let size = Vec2::new(10.0 + icon_w + galley.size().x + 10.0, 24.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    if ui.is_rect_visible(rect) {
        let hovered = enabled && resp.hovered();
        let (fill, edge) = if selected {
            (ACCENT_SOFT, ACCENT)
        } else if hovered {
            (CHROME, MUTED)
        } else {
            (PAPER, RULE)
        };
        let painter = ui.painter();
        painter.rect(rect, 4.0, fill, Stroke::new(1.0_f32, edge));

        let mut x = rect.min.x + 10.0;
        if let Some(ic) = icon {
            let ir = Rect::from_min_size(Pos2::new(x, rect.center().y - 8.0), Vec2::splat(16.0));
            paint(painter, ir, ic, color);
            x += icon_w;
        }
        let y = rect.center().y - galley.size().y / 2.0;
        painter.galley(Pos2::new(x, y), galley, color);
    }
    resp
}

/// A full-width selectable row for side panels: icon on the left, label after it.
pub fn row(ui: &mut egui::Ui, icon: Option<Icon>, text: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if selected {
            ACCENT_SOFT
        } else if resp.hovered() {
            CHROME
        } else {
            Color32::TRANSPARENT
        };
        let color = if selected { ACCENT } else { INK };
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, fill);

        let mut x = rect.min.x + 8.0;
        if let Some(ic) = icon {
            let ir = Rect::from_min_size(Pos2::new(x, rect.center().y - 8.0), Vec2::splat(16.0));
            paint(painter, ir, ic, color);
            x += 22.0;
        }
        let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(13.0), color);
        let y = rect.center().y - galley.size().y / 2.0;
        painter.galley(Pos2::new(x, y), galley, color);
    }
    resp
}

/// Icon colour in the toolbar and menus.
pub const ICON: Color32 = Color32::from_rgb(0x44, 0x47, 0x46);
const HOVER: Color32 = Color32::from_rgb(0xE8, 0xEA, 0xED);

/// Round icon button used in the toolbar. `tip` shows on hover.
pub fn button(ui: &mut egui::Ui, icon: Icon, tip: &str, enabled: bool, selected: bool) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(32.0), sense);
    if ui.is_rect_visible(rect) {
        let hovered = enabled && resp.hovered();
        let fill = if selected {
            theme::ACCENT_SOFT
        } else if hovered {
            HOVER
        } else {
            Color32::TRANSPARENT
        };
        let color = if !enabled {
            DISABLED
        } else if selected {
            ACCENT
        } else {
            ICON
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 16.0, fill);
        paint(painter, Rect::from_center_size(rect.center(), Vec2::splat(18.0)), icon, color);
    }
    resp.on_hover_text(tip)
}

/// Round letter button (B, I, U) for text styles.
pub fn letter_button(ui: &mut egui::Ui, letter: &str, tip: &str, enabled: bool, selected: bool, underline: bool) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(32.0), sense);
    if ui.is_rect_visible(rect) {
        let hovered = enabled && resp.hovered();
        let fill = if selected {
            theme::ACCENT_SOFT
        } else if hovered {
            HOVER
        } else {
            Color32::TRANSPARENT
        };
        let color = if !enabled {
            DISABLED
        } else if selected {
            ACCENT
        } else {
            ICON
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 16.0, fill);
        let font = egui::FontId::proportional(15.0);
        // Faux bold: draw twice with a small horizontal offset.
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, letter, font.clone(), color);
        if underline {
            painter.line_segment(
                [rect.center() + Vec2::new(-5.0, 8.0), rect.center() + Vec2::new(5.0, 8.0)],
                Stroke::new(1.0_f32, color),
            );
        }
    }
    resp.on_hover_text(tip)
}

/// Filled blue pill button, for the main action.
pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), egui::FontId::proportional(13.0), Color32::WHITE);
    let size = Vec2::new(galley.size().x + 36.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if resp.hovered() {
            Color32::from_rgb(0x18, 0x63, 0xC9)
        } else {
            ACCENT
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 16.0, fill);
        painter.galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    }
    resp
}

/// Underlined text tab. Returns true when clicked.
pub fn text_tab(ui: &mut egui::Ui, active: bool, label: &str) -> bool {
    let mut text = egui::RichText::new(label).color(if active { ACCENT } else { MUTED });
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

/// Menu-bar label (File, Edit, ...) that opens a dropdown. Looks like plain text until hovered.
pub fn menu(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals;
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.open] {
            w.bg_fill = Color32::TRANSPARENT;
            w.weak_bg_fill = Color32::TRANSPARENT;
            w.bg_stroke = Stroke::NONE;
        }
        v.widgets.hovered.weak_bg_fill = HOVER;
        v.widgets.hovered.bg_fill = HOVER;
        v.widgets.open.weak_bg_fill = HOVER;
        v.widgets.open.bg_fill = HOVER;
        ui.menu_button(egui::RichText::new(label).size(13.0).color(theme::INK), add);
    });
}

/// Draws the app mark: an accent rounded square with a white page.
pub fn logo(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 7.0, ACCENT);
    let page = Rect::from_center_size(rect.center(), rect.size() * 0.52);
    painter.rect_filled(page, 1.5, Color32::WHITE);
    for i in 0..3 {
        let y = page.min.y + page.height() * (0.35 + 0.2 * i as f32);
        painter.line_segment(
            [egui::pos2(page.min.x + 3.0, y), egui::pos2(page.max.x - 3.0, y)],
            Stroke::new(1.2_f32, ACCENT),
        );
    }
}

/// The application icon: a white page on the accent colour, with three text lines.
pub fn app_icon() -> egui::IconData {
    const N: u32 = 64;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    let accent = [ACCENT.r(), ACCENT.g(), ACCENT.b(), 255];
    let white = [255, 255, 255, 255];

    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let px = if !in_rounded_rect(fx, fy, 0.0, 0.0, 64.0, 64.0, 14.0) {
                [0, 0, 0, 0]
            } else if in_rect(fx, fy, 18.0, 12.0, 46.0, 52.0) {
                let on_line = [24.0_f32, 32.0, 40.0]
                    .iter()
                    .any(|&ly| in_rect(fx, fy, 24.0, ly - 1.5, 40.0, ly + 1.5));
                if on_line { accent } else { white }
            } else {
                accent
            };
            rgba.extend_from_slice(&px);
        }
    }

    egui::IconData { rgba, width: N, height: N }
}

fn in_rect(fx: f32, fy: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    fx >= x0 && fx <= x1 && fy >= y0 && fy <= y1
}

fn in_rounded_rect(fx: f32, fy: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    if !in_rect(fx, fy, x0, y0, x1, y1) {
        return false;
    }
    let dx = (x0 + r - fx).max(fx - (x1 - r)).max(0.0);
    let dy = (y0 + r - fy).max(fy - (y1 - r)).max(0.0);
    dx * dx + dy * dy <= r * r
}
