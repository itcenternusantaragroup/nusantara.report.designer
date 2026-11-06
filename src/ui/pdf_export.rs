//! Vector PDF export of the printed sheets.
//!
//! Uses the same pagination and the same evaluated values as the preview, so the PDF matches
//! what the preview shows. Text is set in Arial when it is found in the Windows font folder
//! (embedded in the PDF, Unicode-safe, and measured for word wrap); otherwise Helvetica is used
//! and text widths are estimated.

use std::path::PathBuf;

use printpdf::{
    BuiltinFont, Color, Image, ImageTransform, IndirectFontRef, Line, Mm, PdfDocument,
    PdfDocumentReference, PdfLayerReference, Point, Polygon, Pt, Rgb,
};
use printpdf::path::{PaintMode, WindingOrder};
use printpdf::TextMatrix;
use crate::model::report::Watermark;

use crate::model::expr::EvalContext;
use crate::model::richtext::parse_runs;
use crate::model::*;
use super::designer::ReportDesignerApp;
use super::preview::{base_vars, build_sheets, page_size};

const PT_PER_MM: f32 = 72.0 / 25.4;
const MM_PER_PT: f32 = 25.4 / 72.0;
/// Resolution used to place embedded pictures.
const IMAGE_DPI: f32 = 300.0;

/// Builds the whole report as one PDF document, one PDF page per printed sheet.
pub fn sheets_to_pdf(app: &ReportDesignerApp) -> Result<Vec<u8>, String> {
    let sheets = build_sheets(app);
    if sheets.is_empty() {
        return Err("Nothing to export.".to_string());
    }
    let total = sheets.len();
    let vars = base_vars(app);

    let doc = PdfDocument::empty(app.report.alias.clone());
    let fonts = load_fonts(&doc)?;

    for (n, sheet) in sheets.iter().enumerate() {
        let (pw, ph) = page_size(app, sheet);
        let (page, layer_idx) = doc.add_page(Mm(pw), Mm(ph), "Page");
        let layer = doc.get_page(page).get_layer(layer_idx);

        let mut sheet_vars = vars.clone();
        sheet_vars.insert("PageNumber".to_string(), (n + 1).to_string());
        sheet_vars.insert("TotalPageCount".to_string(), total.to_string());

        if let Some(wm) = &app.report.pages[sheet.page_idx].watermark {
            draw_watermark(&layer, &fonts, pw, ph, wm);
        }

        for item in &sheet.items {
            let eval = EvalContext {
                dictionary: &app.report.dictionary,
                row_index: &item.rows,
                vars: &sheet_vars,
            };
            let mut comp = item.comp.clone();
            if matches!(comp.comp_type, ComponentType::Text | ComponentType::Barcode | ComponentType::CheckBox) {
                comp.text = eval.evaluate(&comp.text);
            }
            let comp = comp.with_conditions(&comp.text);
            draw_component(&layer, &fonts, ph, &comp);
        }
    }

    doc.save_to_bytes().map_err(|e| format!("{e:?}"))
}

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// One font face. `data` is kept when it is a TrueType file, so text can be measured.
struct FontFace {
    font: IndirectFontRef,
    data: Option<Vec<u8>>,
}

/// Faces indexed by style bits: bit 0 = bold, bit 1 = italic.
struct Fonts {
    faces: Vec<FontFace>,
}

fn load_fonts(doc: &PdfDocumentReference) -> Result<Fonts, String> {
    let dir = std::env::var("WINDIR")
        .map(|w| PathBuf::from(w).join("Fonts"))
        .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\Fonts"));
    // Same order as the style bits: regular, bold, italic, bold italic.
    let names = ["arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"];

    let mut faces = Vec::new();
    if names.iter().all(|n| dir.join(n).is_file()) {
        for name in names {
            let data = std::fs::read(dir.join(name)).map_err(|e| e.to_string())?;
            let font = doc.add_external_font(&data[..]).map_err(|e| format!("{e:?}"))?;
            faces.push(FontFace { font, data: Some(data) });
        }
    } else {
        let builtins = [
            BuiltinFont::Helvetica,
            BuiltinFont::HelveticaBold,
            BuiltinFont::HelveticaOblique,
            BuiltinFont::HelveticaBoldOblique,
        ];
        for builtin in builtins {
            let font = doc.add_builtin_font(builtin).map_err(|e| format!("{e:?}"))?;
            faces.push(FontFace { font, data: None });
        }
    }
    Ok(Fonts { faces })
}

/// Width of `text` in points at `size` pt.
fn width_pt(face: &FontFace, text: &str, size: f32) -> f32 {
    if let Some(data) = &face.data {
        if let Ok(parsed) = ttf_parser::Face::parse(data, 0) {
            let upem = parsed.units_per_em() as f32;
            let w: f32 = text
                .chars()
                .map(|ch| {
                    parsed
                        .glyph_index(ch)
                        .and_then(|g| parsed.glyph_hor_advance(g))
                        .map(f32::from)
                        .unwrap_or(upem * 0.5)
                })
                .sum();
            return w / upem * size;
        }
    }
    text.chars().count() as f32 * 0.5 * size
}

// ---------------------------------------------------------------------------
// Text layout
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Word,
    Space,
    Break,
}

/// A piece of text with one style, placed on one line.
#[derive(Clone)]
struct Seg {
    text: String,
    kind: Kind,
    style: usize,
    size: f32,
    color: [u8; 4],
    underline: bool,
    /// Width in points.
    width: f32,
}

/// Splits text into words, spaces and hard line breaks.
fn pieces(text: &str) -> Vec<(String, Kind)> {
    let mut out: Vec<(String, Kind)> = Vec::new();
    for ch in text.chars() {
        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            out.push((String::new(), Kind::Break));
            continue;
        }
        let ch = if ch == '\t' { ' ' } else { ch };
        let kind = if ch == ' ' { Kind::Space } else { Kind::Word };
        match out.last_mut() {
            Some((s, k)) if *k == kind => s.push(ch),
            _ => out.push((ch.to_string(), kind)),
        }
    }
    out
}

/// The text of a component as styled segments, following its HTML runs when allowed.
fn runs_to_segs(comp: &Component, text: &str, fonts: &Fonts) -> Vec<Seg> {
    let mut segs = Vec::new();
    for run in parse_runs(text, comp.allow_html_tags) {
        let style = (run.bold || comp.bold) as usize | (((run.italic || comp.italic) as usize) << 1);
        let size = run.size_pt.unwrap_or(comp.font_size * run.size_scale);
        let color = run.color.unwrap_or(comp.text_color);
        let underline = run.underline || comp.underline;
        for (piece, kind) in pieces(&run.text) {
            let width = if kind == Kind::Break { 0.0 } else { width_pt(&fonts.faces[style], &piece, size) };
            segs.push(Seg { text: piece, kind, style, size, color, underline, width });
        }
    }
    segs
}

/// Splits segments into lines. With `max_w` (points) the lines are word-wrapped.
fn layout_lines(segs: Vec<Seg>, max_w: Option<f32>) -> Vec<Vec<Seg>> {
    let mut lines: Vec<Vec<Seg>> = vec![Vec::new()];
    let mut cur = 0.0_f32;
    for seg in segs {
        if seg.kind == Kind::Break {
            lines.push(Vec::new());
            cur = 0.0;
            continue;
        }
        if let Some(max) = max_w {
            if seg.kind == Kind::Space && cur == 0.0 {
                continue;
            }
            if seg.kind == Kind::Word && cur > 0.0 && cur + seg.width > max {
                if let Some(line) = lines.last_mut() {
                    while line.last().is_some_and(|s| s.kind == Kind::Space) {
                        line.pop();
                    }
                }
                lines.push(Vec::new());
                cur = 0.0;
            }
        }
        cur += seg.width;
        if let Some(line) = lines.last_mut() {
            line.push(seg);
        }
    }
    lines
}

/// Draws text inside a box (mm, top-left origin on the page). `base` is the component font size.
#[allow(clippy::too_many_arguments)]
fn draw_text(
    layer: &PdfLayerReference,
    fonts: &Fonts,
    ph: f32,
    segs: Vec<Seg>,
    base: f32,
    (x, y, w, h): (f32, f32, f32, f32),
    align_h: AlignH,
    align_v: AlignV,
    wrap: bool,
) {
    let box_w = w * PT_PER_MM;
    let lines = layout_lines(segs, wrap.then_some(box_w));
    let line_size = |line: &[Seg]| {
        let m = line.iter().map(|s| s.size).fold(0.0_f32, f32::max);
        if m > 0.0 { m } else { base }
    };
    let total: f32 = lines.iter().map(|l| line_size(l) * 1.15).sum();
    let box_h = h * PT_PER_MM;
    let mut top = match align_v {
        AlignV::Top => 0.0,
        AlignV::Center => (box_h - total) / 2.0,
        AlignV::Bottom => box_h - total,
    };

    for line in &lines {
        let size = line_size(line);
        let line_w: f32 = line.iter().map(|s| s.width).sum();
        let mut cx = match align_h {
            AlignH::Center => (box_w - line_w) / 2.0,
            AlignH::Right => box_w - line_w,
            _ => 0.0,
        };
        let baseline_mm = y + (top + size * 0.905) * MM_PER_PT;
        for seg in line {
            layer.set_fill_color(rgb(seg.color));
            layer.use_text(
                seg.text.clone(),
                seg.size,
                Mm(x + cx * MM_PER_PT),
                Mm(ph - baseline_mm),
                &fonts.faces[seg.style].font,
            );
            if seg.underline {
                let uy = baseline_mm + seg.size * 0.12 * MM_PER_PT;
                stroke_line(layer, ph, (x + cx * MM_PER_PT, uy), (x + (cx + seg.width) * MM_PER_PT, uy), seg.color, 0.4);
            }
            cx += seg.width;
        }
        top += size * 1.15;
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn rgb(c: [u8; 4]) -> Color {
    Color::Rgb(Rgb::new(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, None))
}

/// A page point (mm, top-left origin) as a PDF point (bottom-left origin).
fn pt_at(ph: f32, x: f32, y: f32) -> Point {
    Point { x: Pt(x * PT_PER_MM), y: Pt((ph - y) * PT_PER_MM) }
}

fn poly(ph: f32, pts: &[(f32, f32)], mode: PaintMode) -> Polygon {
    Polygon {
        rings: vec![pts.iter().map(|&(x, y)| (pt_at(ph, x, y), false)).collect()],
        mode,
        winding_order: WindingOrder::NonZero,
    }
}

fn rect_poly(ph: f32, x: f32, y: f32, w: f32, h: f32, mode: PaintMode) -> Polygon {
    poly(ph, &[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], mode)
}

fn stroke_line(layer: &PdfLayerReference, ph: f32, a: (f32, f32), b: (f32, f32), color: [u8; 4], width_pt: f32) {
    layer.set_outline_color(rgb(color));
    layer.set_outline_thickness(width_pt);
    layer.add_line(Line {
        points: vec![(pt_at(ph, a.0, a.1), false), (pt_at(ph, b.0, b.1), false)],
        is_closed: false,
    });
}

fn border_pt(comp: &Component) -> f32 {
    (comp.border_width * PT_PER_MM).max(0.5)
}

fn draw_component(layer: &PdfLayerReference, fonts: &Fonts, ph: f32, comp: &Component) {
    let (x, y, w, h) = (comp.rect.x, comp.rect.y, comp.rect.w, comp.rect.h);

    if comp.bg_color[3] > 0 {
        layer.set_fill_color(rgb(comp.bg_color));
        layer.add_polygon(rect_poly(ph, x, y, w, h, PaintMode::Fill));
    }
    if comp.border_sides.has_any() {
        let width = border_pt(comp);
        let s = &comp.border_sides;
        let c = comp.border_color;
        if s.top { stroke_line(layer, ph, (x, y), (x + w, y), c, width); }
        if s.bottom { stroke_line(layer, ph, (x, y + h), (x + w, y + h), c, width); }
        if s.left { stroke_line(layer, ph, (x, y), (x, y + h), c, width); }
        if s.right { stroke_line(layer, ph, (x + w, y), (x + w, y + h), c, width); }
    }

    match comp.comp_type {
        ComponentType::Text => {
            let segs = runs_to_segs(comp, &comp.text, fonts);
            draw_text(layer, fonts, ph, segs, comp.font_size, (x, y, w, h), comp.align_h, comp.align_v, comp.word_wrap);
        }
        ComponentType::Barcode => {
            let text_h = 3.5;
            let (bx, bw) = (x + 1.0, (w - 2.0).max(1.0));
            let bar_h = (h - text_h).max(1.0);
            match crate::model::barcode::bars(&comp.barcode_format, &comp.text) {
                Ok(bars) => {
                    layer.set_fill_color(rgb([0, 0, 0, 255]));
                    for (start, width) in bars {
                        layer.add_polygon(rect_poly(ph, bx + start * bw, y + 1.0, width * bw, bar_h - 1.0, PaintMode::Fill));
                    }
                }
                Err(_) => {}
            }
            let segs = vec![Seg {
                text: comp.text.clone(),
                kind: Kind::Word,
                style: 0,
                size: 9.0,
                color: [0, 0, 0, 255],
                underline: false,
                width: width_pt(&fonts.faces[0], &comp.text, 9.0),
            }];
            draw_text(layer, fonts, ph, segs, 9.0, (x + 1.0, y, w - 2.0, h), AlignH::Center, AlignV::Bottom, false);
        }        ComponentType::Line => {
            let width = border_pt(comp);
            if w < h {
                stroke_line(layer, ph, (x + w / 2.0, y), (x + w / 2.0, y + h), comp.border_color, width);
            } else {
                stroke_line(layer, ph, (x, y + h / 2.0), (x + w, y + h / 2.0), comp.border_color, width);
            }
        }
        ComponentType::Shape => {
            let filled = comp.bg_color[3] > 0;
            let mode = if filled { PaintMode::FillStroke } else { PaintMode::Stroke };
            if filled {
                layer.set_fill_color(rgb(comp.bg_color));
            }
            layer.set_outline_color(rgb(comp.border_color));
            layer.set_outline_thickness(border_pt(comp));
            match comp.shape_kind {
                ShapeKind::Rectangle | ShapeKind::RoundedRectangle => {
                    layer.add_polygon(rect_poly(ph, x, y, w, h, mode));
                }
                ShapeKind::Ellipse => {
                    // The preview draws a circle with the smaller side as its diameter.
                    let r = w.min(h) / 2.0;
                    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                    let pts: Vec<(f32, f32)> = (0..72)
                        .map(|k| {
                            let t = k as f32 / 72.0 * std::f32::consts::TAU;
                            (cx + r * t.cos(), cy + r * t.sin())
                        })
                        .collect();
                    layer.add_polygon(poly(ph, &pts, mode));
                }
                ShapeKind::Triangle => {
                    layer.add_polygon(poly(ph, &[(x + w / 2.0, y), (x + w, y + h), (x, y + h)], mode));
                }
            }
        }
        ComponentType::CheckBox => {
            let side = h.min(4.0);
            let by = y + h / 2.0 - side / 2.0;
            layer.set_fill_color(rgb([255, 255, 255, 255]));
            layer.set_outline_color(rgb([0, 0, 0, 255]));
            layer.set_outline_thickness(1.0);
            layer.add_polygon(rect_poly(ph, x, by, side, side, PaintMode::FillStroke));
            if comp.checked {
                let p = |fx: f32, fy: f32| (x + fx * side, by + fy * side);
                let width = (side * 0.12 * PT_PER_MM).max(1.0);
                stroke_line(layer, ph, p(0.2, 0.5), p(0.42, 0.75), [0, 0, 0, 255], width);
                stroke_line(layer, ph, p(0.42, 0.75), p(0.82, 0.25), [0, 0, 0, 255], width);
            }
            let text_x = x + side + 0.8;
            let segs = runs_to_segs(comp, &comp.text, fonts);
            draw_text(layer, fonts, ph, segs, comp.font_size, (text_x, y, w - side - 0.8, h), AlignH::Left, AlignV::Center, false);
        }
        ComponentType::Image => {
            if let Some(bytes) = &comp.image_data {
                if let Ok(img) = printpdf::image_crate::load_from_memory(bytes) {
                    let (px_w, px_h) = (img.width() as f32, img.height() as f32);
                    // The image is placed at IMAGE_DPI, then scaled to the component size.
                    let nat_w = px_w * 72.0 / IMAGE_DPI;
                    let nat_h = px_h * 72.0 / IMAGE_DPI;
                    let pic = Image::from_dynamic_image(&img);
                    pic.add_to_layer(
                        layer.clone(),
                        ImageTransform {
                            translate_x: Some(Mm(x)),
                            translate_y: Some(Mm(ph - y - h)),
                            rotate: None,
                            scale_x: Some(w * PT_PER_MM / nat_w),
                            scale_y: Some(h * PT_PER_MM / nat_h),
                            dpi: Some(IMAGE_DPI),
                        },
                    );
                }
            }
        }
        // Tables are not rendered yet, as in the preview.
        ComponentType::Table => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str) -> Seg {
        let (t, kind) = pieces(text).remove(0);
        let width = if kind == Kind::Space { 5.0 } else { text.chars().count() as f32 * 10.0 };
        Seg { text: t, kind, style: 0, size: 10.0, color: [0, 0, 0, 255], underline: false, width }
    }

    #[test]
    fn word_wrap_breaks_at_spaces_and_hard_breaks() {
        let segs: Vec<Seg> = ["aa", " ", "bb", " ", "cc", "\n", "dd"]
            .iter()
            .map(|t| if *t == "\n" { Seg { text: String::new(), kind: Kind::Break, style: 0, size: 10.0, color: [0; 4], underline: false, width: 0.0 } } else { seg(t) })
            .collect();
        let lines = layout_lines(segs, Some(45.0));
        let texts: Vec<String> = lines.iter().map(|l| l.iter().map(|s| s.text.as_str()).collect()).collect();
        assert_eq!(texts, vec!["aa bb".to_string(), "cc".to_string(), "dd".to_string()]);
    }

    #[test]
    fn sample_report_exports_to_pdf() {
        let Ok(path) = std::env::var("NUSA_PREVIEW_FILE") else { return };
        let app = ReportDesignerApp::new(None, Some(path));
        let bytes = sheets_to_pdf(&app).expect("export");
        assert!(bytes.starts_with(b"%PDF"));
        let out = std::env::temp_dir().join("nusa_sample_report.pdf");
        std::fs::write(&out, &bytes).expect("write");
        println!("pdf bytes={} path={}", bytes.len(), out.display());
    }
}
/// The watermark text, centred on the page and rotated (clockwise by its angle).
/// Its alpha is blended into the colour, since PDF transparency is not written here.
fn draw_watermark(layer: &PdfLayerReference, fonts: &Fonts, pw: f32, ph: f32, wm: &Watermark) {
    let a = wm.color[3] as f32 / 255.0;
    let blend = |c: u8| (255.0 - (255.0 - c as f32) * a) / 255.0;
    let width = width_pt(&fonts.faces[0], &wm.text, wm.font_size);
    let (cx, cy) = (pw * PT_PER_MM / 2.0, ph * PT_PER_MM / 2.0);
    // PDF rotates counter-clockwise; the watermark angle is clockwise on screen.
    let theta = -wm.angle.to_radians();
    // Start of the baseline, relative to the centre of the text.
    let (sx, sy) = (-width / 2.0, -wm.font_size * 0.35);
    let e = cx + sx * theta.cos() - sy * theta.sin();
    let f = cy + sx * theta.sin() + sy * theta.cos();
    layer.begin_text_section();
    layer.set_font(&fonts.faces[0].font, wm.font_size);
    layer.set_fill_color(Color::Rgb(Rgb::new(blend(wm.color[0]), blend(wm.color[1]), blend(wm.color[2]), None)));
    layer.set_text_matrix(TextMatrix::Raw([theta.cos(), theta.sin(), -theta.sin(), theta.cos(), e, f]));
    layer.write_text(wm.text.clone(), &fonts.faces[0].font);
    layer.end_text_section();
}
#[cfg(test)]
mod demo_tests {
    use super::*;

    #[test]
    fn demo_watermark_and_barcode_pdf() {
        let Ok(out) = std::env::var("NUSA_DEMO_PDF") else { return };
        let mut app = ReportDesignerApp::new(None, None);
        let page = &mut app.report.pages[0];
        page.watermark = Some(crate::model::Watermark::default());
        let mut bc = Component::default();
        bc.comp_type = ComponentType::Barcode;
        bc.barcode_format = "Code128".into();
        bc.text = "NO-2026-00123".into();
        bc.rect = RectMm::new(20.0, 30.0, 90.0, 25.0);
        page.components.push(bc);
        let mut cond = Component::default();
        cond.text = "{Value}".into();
        cond.rect = RectMm::new(20.0, 70.0, 60.0, 8.0);
        cond.conditions = vec![CondRule { op: CondOp::Greater, value: "100".into(), bg_color: [255, 235, 156, 255], text_color: [156, 0, 6, 255] }];
        page.components.push(cond);
        let bytes = sheets_to_pdf(&app).expect("export");
        std::fs::write(&out, bytes).expect("write");
    }
}