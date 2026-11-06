//! Draws a text component with its rich text (styles, paragraphs, alignment, lists) laid out by
//! egui. Used by the canvas and by the preview so both show the same result.

use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Align, Color32, FontFamily, FontId, Galley, Painter, Rect, Stroke};

use crate::model::richtext::{parse_paragraphs, Paragraph, Run};
use crate::model::{AlignH, AlignV, Component};

/// Pixels per point at 96 DPI, scaled by the view zoom.
pub fn px_per_pt(zoom: f32) -> f32 {
    1.333 * zoom
}

/// The whole text as one layout, paragraphs one under another. Used by the editor preview.
pub fn layout_job(comp: &Component, text: &str, px_per_pt: f32, wrap_width: Option<f32>) -> LayoutJob {
    let base = to_color(comp.text_color);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width.unwrap_or(f32::INFINITY);
    job.halign = align_of(comp.align_h, wrap_width.is_some());
    let paragraphs = parse_paragraphs(text, comp.allow_html_tags);
    for (i, para) in paragraphs.iter().enumerate() {
        if i > 0 {
            job.append("\n", 0.0, run_format(comp, &Run::default(), base, px_per_pt));
        }
        append_paragraph(&mut job, comp, para, px_per_pt, base);
    }
    job
}

/// One paragraph as its own layout, with its alignment (its own, or the component's).
fn paragraph_job(comp: &Component, para: &Paragraph, px_per_pt: f32, wrap_width: Option<f32>) -> (LayoutJob, AlignH) {
    let align = para.align.unwrap_or(comp.align_h);
    let base = to_color(comp.text_color);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width.unwrap_or(f32::INFINITY);
    job.halign = align_of(align, wrap_width.is_some());
    append_paragraph(&mut job, comp, para, px_per_pt, base);
    (job, align)
}

/// Adds the list marker (if any) and the runs of a paragraph. An empty paragraph gets a space,
/// so it still takes up a line.
fn append_paragraph(job: &mut LayoutJob, comp: &Component, para: &Paragraph, px_per_pt: f32, base: Color32) {
    if let Some(bullet) = para.bullet {
        let marker = Run { text: bullet.label(), ..Run::default() };
        let style = para.runs.first().unwrap_or(&marker);
        let format = run_format(comp, style, base, px_per_pt);
        job.append(&bullet.label(), 0.0, format);
    }
    if para.runs.is_empty() {
        job.append(" ", 0.0, run_format(comp, &Run::default(), base, px_per_pt));
    }
    for run in &para.runs {
        job.append(&run.text, 0.0, run_format(comp, run, base, px_per_pt));
    }
}

fn align_of(align: AlignH, wrapped: bool) -> Align {
    // Horizontal alignment inside the text block only matters when the text wraps to its width.
    // Without wrapping, `draw` positions each paragraph itself.
    if !wrapped {
        return Align::LEFT;
    }
    match align {
        AlignH::Center => Align::Center,
        AlignH::Right => Align::RIGHT,
        _ => Align::LEFT,
    }
}

fn run_format(comp: &Component, run: &Run, base_color: Color32, px_per_pt: f32) -> TextFormat {
    let size_pt = run.size_pt.unwrap_or(comp.font_size * run.size_scale);
    let bold = comp.bold || run.bold;
    let italic = comp.italic || run.italic;
    let underline = comp.underline || run.underline;
    let color = run.color.map(to_color).unwrap_or(base_color);
    let monospace = run.family.as_deref().map_or(false, |f| {
        let f = f.to_ascii_lowercase();
        f.contains("courier") || f.contains("consolas") || f.contains("mono")
    });
    let family = if monospace {
        FontFamily::Monospace
    } else if bold {
        FontFamily::Name("bold".into())
    } else {
        FontFamily::Proportional
    };
    TextFormat {
        font_id: FontId::new(size_pt * px_per_pt, family),
        color,
        italics: italic,
        underline: if underline { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
        strikethrough: if run.strike { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
        background: run.highlight.map(to_color).unwrap_or(Color32::TRANSPARENT),
        ..Default::default()
    }
}

/// Draws `text` inside `rect` according to the component's alignment and word wrap.
pub fn draw(painter: &Painter, comp: &Component, text: &str, rect: Rect, px_per_pt: f32) {
    let inner = rect.shrink2(egui::vec2(3.0, 2.0));
    let wrap = if comp.word_wrap { Some(inner.width()) } else { None };

    // One galley per paragraph, so each can have its own alignment.
    let blocks: Vec<(Arc<Galley>, AlignH)> = parse_paragraphs(text, comp.allow_html_tags)
        .iter()
        .map(|para| {
            let (job, align) = paragraph_job(comp, para, px_per_pt, wrap);
            (painter.layout_job(job), align)
        })
        .collect();
    let total: f32 = blocks.iter().map(|(g, _)| g.size().y).sum();

    let mut y = match comp.align_v {
        AlignV::Center => inner.center().y - total / 2.0,
        AlignV::Bottom => inner.bottom() - total,
        _ => inner.top(),
    };

    // Clip only horizontally: text taller than its box stays visible instead of being cut at the
    // top and bottom (the report's header boxes are often shorter than their font).
    let clip = Rect::from_min_max(egui::pos2(rect.min.x, -1.0e6), egui::pos2(rect.max.x, 1.0e6));
    for (galley, align) in blocks {
        let size = galley.size();
        // egui aligns wrapped lines around the galley origin (centre for Center, right edge for
        // Right), so the origin is the anchor point. Unwrapped text is positioned from its width.
        let x = if comp.word_wrap {
            match align {
                AlignH::Center => inner.center().x,
                AlignH::Right => inner.right(),
                _ => inner.left(),
            }
        } else {
            match align {
                AlignH::Center => inner.center().x - size.x / 2.0,
                AlignH::Right => inner.right() - size.x,
                _ => inner.left(),
            }
        };
        painter
            .with_clip_rect(clip)
            .galley(egui::pos2(x, y), galley, to_color(comp.text_color));
        y += size.y;
    }
}

/// The height the text takes when drawn at `width` (wrapped if the component wraps). The editor
/// uses it to size its preview.
pub fn measure(ctx: &egui::Context, comp: &Component, text: &str, width: f32, px_per_pt: f32) -> f32 {
    let wrap = if comp.word_wrap { Some(width) } else { None };
    parse_paragraphs(text, comp.allow_html_tags)
        .iter()
        .map(|para| {
            let (job, _) = paragraph_job(comp, para, px_per_pt, wrap);
            ctx.fonts(|f| f.layout_job(job).size().y)
        })
        .sum()
}
pub fn to_color(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}
