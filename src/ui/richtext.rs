//! Draws a text component, with its HTML runs (bold, italic, colour, size, line breaks)
//! laid out by egui. Used by the canvas and by the preview so both show the same result.

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Align, Color32, FontFamily, FontId, Painter, Rect, Stroke};

use crate::model::richtext::{parse_runs, Run};
use crate::model::{AlignH, Component};

/// Pixels per point at 96 DPI, scaled by the view zoom.
pub fn px_per_pt(zoom: f32) -> f32 {
    1.333 * zoom
}

/// Builds the text layout for `text`, honouring the component's font, colour and HTML flag.
pub fn layout_job(comp: &Component, text: &str, px_per_pt: f32, wrap_width: Option<f32>) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width.unwrap_or(f32::INFINITY);
    // Horizontal alignment inside the text block only matters when the text wraps to its width.
    // Without wrapping, `draw` positions the whole block itself.
    job.halign = match (wrap_width.is_some(), comp.align_h) {
        (true, AlignH::Center) => Align::Center,
        (true, AlignH::Right) => Align::RIGHT,
        _ => Align::LEFT,
    };

    // A trailing line break does not start a visible line (Stimulsoft ignores it). Keeping it
    // would make a centred, single-line text sit higher than its neighbours.
    let text = text.trim_end_matches(['\r', '\n']);
    let base_color = to_color(comp.text_color);
    for run in parse_runs(text, comp.allow_html_tags) {
        let format = run_format(comp, &run, base_color, px_per_pt);
        job.append(&run.text, 0.0, format);
    }
    job
}

fn run_format(comp: &Component, run: &Run, base_color: Color32, px_per_pt: f32) -> TextFormat {
    let size_pt = run.size_pt.unwrap_or(comp.font_size * run.size_scale);
    let bold = comp.bold || run.bold;
    let italic = comp.italic || run.italic;
    let underline = comp.underline || run.underline;
    let color = run.color.map(to_color).unwrap_or(base_color);
    TextFormat {
        font_id: FontId::new(
            size_pt * px_per_pt,
            if bold { FontFamily::Name("bold".into()) } else { FontFamily::Proportional },
        ),
        color,
        italics: italic,
        underline: if underline { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
        ..Default::default()
    }
}

/// Draws `text` inside `rect` according to the component's alignment and word wrap.
pub fn draw(painter: &Painter, comp: &Component, text: &str, rect: Rect, px_per_pt: f32) {
    let inner = rect.shrink2(egui::vec2(3.0, 2.0));
    let wrap = if comp.word_wrap { Some(inner.width()) } else { None };
    let job = layout_job(comp, text, px_per_pt, wrap);
    let galley = painter.layout_job(job);
    let size = galley.size();

    // egui aligns wrapped lines around the galley origin (centre for Center, right edge for Right),
    // so the origin is the anchor point. Unwrapped text is positioned from its own width.
    let x = if comp.word_wrap {
        match comp.align_h {
            AlignH::Center => inner.center().x,
            AlignH::Right => inner.right(),
            _ => inner.left(),
        }
    } else {
        match comp.align_h {
            AlignH::Center => inner.center().x - size.x / 2.0,
            AlignH::Right => inner.right() - size.x,
            _ => inner.left(),
        }
    };
    let y = match comp.align_v {
        crate::model::AlignV::Center => inner.center().y - size.y / 2.0,
        crate::model::AlignV::Bottom => inner.bottom() - size.y,
        _ => inner.top(),
    };

    // Clip only horizontally: text taller than its box stays visible instead of being cut at the
    // top and bottom (the report's header boxes are often shorter than their font).
    let clip = Rect::from_min_max(
        egui::pos2(rect.min.x, -1.0e6),
        egui::pos2(rect.max.x, 1.0e6),
    );
    painter
        .with_clip_rect(clip)
        .galley(egui::pos2(x, y), galley, to_color(comp.text_color));
}

pub fn to_color(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}
