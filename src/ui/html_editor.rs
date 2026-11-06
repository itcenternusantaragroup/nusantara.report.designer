//! Editor for text that uses HTML tags. A toolbar wraps the selected text in tags, a source box
//! holds the text, and a preview shows how it will look on the page.

use std::ops::Range;

use eframe::egui::{self, text::CCursor, text_selection::CCursorRange, Color32, FontId, Frame, Margin, Sense, Vec2};

use crate::model::richtext::strip_tags;
use crate::model::Component;

use super::icons;
use super::richtext as rt;
use super::theme::{self, MUTED, PAPER, RULE};

/// Text colours offered in the toolbar, as (label, hex).
const COLORS: [(&str, &str); 6] = [
    ("Black", "#202124"),
    ("Red", "#D93025"),
    ("Blue", "#1A73E8"),
    ("Green", "#188038"),
    ("Orange", "#E37400"),
    ("Grey", "#5F6368"),
];

/// Text sizes offered in the toolbar, as (label, HTML size 1..7).
const SIZES: [(&str, u8); 4] = [("Small", 2), ("Normal", 3), ("Large", 5), ("Huge", 6)];

/// Shows the editor for `comp.text`. Returns true when the text changed.
pub fn show(ui: &mut egui::Ui, comp: &mut Component) -> bool {
    let id = egui::Id::new(("html_editor", comp.id.as_str()));
    let mut changed = false;

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if icons::letter_button(ui, "B", "Bold", true, false, false).clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<b>", "</b>");
        }
        if icons::letter_button(ui, "I", "Italic", true, false, false).clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<i>", "</i>");
        }
        if icons::letter_button(ui, "U", "Underline", true, false, true).clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<u>", "</u>");
        }
        if icons::letter_button(ui, "S", "Strikethrough", true, false, false).clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<s>", "</s>");
        }
        if icons::chip(ui, None, "Highlight", true, false).clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<mark>", "</mark>");
        }

        ui.separator();

        egui::ComboBox::from_id_salt(("html_heading", comp.id.as_str()))
            .width(86.0)
            .selected_text("Heading")
            .show_ui(ui, |ui| {
                for level in 1..=3 {
                    if ui.selectable_label(false, format!("Heading {level}")).clicked() {
                        let (open, close) = (format!("<h{level}>"), format!("</h{level}>"));
                        changed |= wrap_selection(ui.ctx(), id, &mut comp.text, &open, &close);
                    }
                }
            });
        if icons::chip(ui, None, "List", true, false).on_hover_text("Bulleted item").clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<ul><li>", "</li></ul>");
        }
        if icons::chip(ui, None, "Numbered", true, false).on_hover_text("Numbered item").clicked() {
            changed |= wrap_selection(ui.ctx(), id, &mut comp.text, "<ol><li>", "</li></ol>");
        }
        for (label, align) in [("Left", "left"), ("Centre", "center"), ("Right", "right")] {
            if icons::chip(ui, None, label, true, false).on_hover_text("Paragraph alignment").clicked() {
                let open = format!("<p align=\"{align}\">");
                changed |= wrap_selection(ui.ctx(), id, &mut comp.text, &open, "</p>");
            }
        }

        ui.separator();

        for (label, hex) in COLORS {
            let c = parse_hex(hex);
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
            ui.painter().rect_filled(rect, 4.0, c);
            if resp.hovered() {
                ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, MUTED));
            }
            if resp.on_hover_text(label).clicked() {
                let open = format!("<font color=\"{hex}\">");
                changed |= wrap_selection(ui.ctx(), id, &mut comp.text, &open, "</font>");
            }
        }

        ui.separator();

        egui::ComboBox::from_id_salt(("html_size", comp.id.as_str()))
            .width(86.0)
            .selected_text("Size")
            .show_ui(ui, |ui| {
                for (label, size) in SIZES {
                    if ui.selectable_label(false, label).clicked() {
                        let open = format!("<font size=\"{size}\">");
                        changed |= wrap_selection(ui.ctx(), id, &mut comp.text, &open, "</font>");
                    }
                }
            });

        ui.separator();

        if icons::chip(ui, None, "Line break", true, false).clicked() {
            changed |= insert_at_cursor(ui.ctx(), id, &mut comp.text, "<br>");
        }
        if icons::chip(ui, None, "Clear tags", true, false).clicked() {
            comp.text = strip_tags(&comp.text);
            changed = true;
        }
    });

    ui.add_space(4.0);
    let output = egui::TextEdit::multiline(&mut comp.text)
        .id(id)
        .font(FontId::monospace(12.0))
        .desired_rows(5)
        .desired_width(f32::INFINITY)
        .hint_text("Type text. Use the buttons above to add tags.")
        .show(ui);
    changed |= output.response.changed();

    ui.add_space(6.0);
    ui.label(theme::caption("Preview"));
    preview(ui, comp);

    changed
}

/// Draws the component text as it will appear on the page.
fn preview(ui: &mut egui::Ui, comp: &Component) {
    Frame::none()
        .fill(PAPER)
        .stroke(egui::Stroke::new(1.0_f32, RULE))
        .rounding(4.0)
        .inner_margin(Margin::symmetric(8.0, 6.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            // Drawn by the same code as the page, so the preview matches it.
            let width = ui.available_width();
            let mut preview = comp.clone();
            preview.word_wrap = true;
            let px = rt::px_per_pt(1.0);
            let height = rt::measure(ui.ctx(), &preview, &preview.text, width, px).max(18.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
            let painter = ui.painter().clone();
            rt::draw(&painter, &preview, &preview.text, rect, px);
        });
}

/// Puts `open` and `close` around the selected text, or at the cursor when nothing is selected.
/// The new selection covers the wrapped text, so a second click wraps it again.
fn wrap_selection(ctx: &egui::Context, id: egui::Id, text: &mut String, open: &str, close: &str) -> bool {
    let range = selection(ctx, id, text);
    let (start, end) = (byte_index(text, range.start), byte_index(text, range.end));
    text.insert_str(end, close);
    text.insert_str(start, open);

    let open_len = open.chars().count();
    let selected = range.end - range.start;
    store_selection(ctx, id, range.start + open_len, range.start + open_len + selected);
    true
}

/// Inserts `insert` at the cursor (replacing any selection).
fn insert_at_cursor(ctx: &egui::Context, id: egui::Id, text: &mut String, insert: &str) -> bool {
    let range = selection(ctx, id, text);
    let (start, end) = (byte_index(text, range.start), byte_index(text, range.end));
    text.replace_range(start..end, insert);
    let at = range.start + insert.chars().count();
    store_selection(ctx, id, at, at);
    true
}

/// The selected character range in the editor, or the end of the text when it has no state yet.
fn selection(ctx: &egui::Context, id: egui::Id, text: &str) -> Range<usize> {
    let len = text.chars().count();
    egui::TextEdit::load_state(ctx, id)
        .and_then(|state| state.cursor.char_range())
        .map(|r| {
            let [a, b] = r.sorted();
            a.index.min(len)..b.index.min(len)
        })
        .unwrap_or(len..len)
}

fn store_selection(ctx: &egui::Context, id: egui::Id, start: usize, end: usize) {
    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(start), CCursor::new(end))));
        state.store(ctx, id);
    }
}

/// Converts a character index into a byte index of `s`.
fn byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices().nth(char_index).map(|(b, _)| b).unwrap_or(s.len())
}

fn parse_hex(hex: &str) -> Color32 {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}
