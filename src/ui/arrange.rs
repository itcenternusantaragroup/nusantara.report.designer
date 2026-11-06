//! Selecting several components at once, and arranging them: align, distribute, and z-order.
//!
//! The primary selection is `selected_component_id`. Extra selected components are kept in
//! `extra_selected`. Ids that no longer exist on the page are ignored.

use crate::model::*;
use super::designer::ReportDesignerApp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    HCenter,
    Right,
    Top,
    VCenter,
    Bottom,
}

impl ReportDesignerApp {
    /// Every selected component id on the current page, primary first.
    pub fn selected_ids(&self) -> Vec<String> {
        let page = self.current_page();
        let mut ids: Vec<String> = Vec::new();
        if let Some(primary) = &self.selected_component_id {
            if page.components.iter().any(|c| &c.id == primary) {
                ids.push(primary.clone());
            }
        }
        for id in &self.extra_selected {
            if !ids.contains(id) && page.components.iter().any(|c| &c.id == id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    /// Selects one component (replacing the selection), or toggles it in the selection.
    pub fn select_component(&mut self, id: &str, additive: bool) {
        if additive {
            if self.selected_component_id.as_deref() == Some(id) {
                // Removing the primary: the first extra one becomes primary.
                self.selected_component_id = self.extra_selected.first().cloned();
                if !self.extra_selected.is_empty() {
                    self.extra_selected.remove(0);
                }
            } else if let Some(pos) = self.extra_selected.iter().position(|x| x == id) {
                self.extra_selected.remove(pos);
            } else if self.selected_component_id.is_some() {
                self.extra_selected.push(id.to_string());
            } else {
                self.selected_component_id = Some(id.to_string());
            }
        } else {
            self.selected_component_id = Some(id.to_string());
            self.extra_selected.clear();
        }
        self.selected_band_id = None;
    }

    /// Sets the selection to exactly `ids` (used by rubber-band selection).
    pub fn select_many(&mut self, ids: Vec<String>) {
        self.selected_component_id = ids.first().cloned();
        self.extra_selected = ids.into_iter().skip(1).collect();
        self.selected_band_id = None;
    }

    pub fn clear_component_selection(&mut self) {
        self.selected_component_id = None;
        self.extra_selected.clear();
    }

    /// Aligns the selected components to the outermost one (needs two or more).
    pub fn align_selected(&mut self, how: Align) {
        let ids = self.selected_ids();
        if ids.len() < 2 {
            return;
        }
        self.push_undo();
        let rects: Vec<(String, RectMm)> = {
            let page = self.current_page();
            ids.iter()
                .filter_map(|id| page.components.iter().find(|c| &c.id == id).map(|c| (id.clone(), c.rect)))
                .collect()
        };
        let left = rects.iter().map(|(_, r)| r.x).fold(f32::MAX, f32::min);
        let right = rects.iter().map(|(_, r)| r.x + r.w).fold(f32::MIN, f32::max);
        let top = rects.iter().map(|(_, r)| r.y).fold(f32::MAX, f32::min);
        let bottom = rects.iter().map(|(_, r)| r.y + r.h).fold(f32::MIN, f32::max);
        let hc = (left + right) / 2.0;
        let vc = (top + bottom) / 2.0;

        let page = self.current_page_mut();
        for (id, _) in &rects {
            if let Some(c) = page.components.iter_mut().find(|c| &c.id == id) {
                match how {
                    Align::Left => c.rect.x = left,
                    Align::Right => c.rect.x = right - c.rect.w,
                    Align::HCenter => c.rect.x = hc - c.rect.w / 2.0,
                    Align::Top => c.rect.y = top,
                    Align::Bottom => c.rect.y = bottom - c.rect.h,
                    Align::VCenter => c.rect.y = vc - c.rect.h / 2.0,
                }
            }
        }
        self.status_message = "Aligned the selection.".to_string();
        self.refresh_code_buffer();
    }

    /// Spreads the selected components evenly between the outermost two (needs three or more).
    pub fn distribute_selected(&mut self, horizontal: bool) {
        let ids = self.selected_ids();
        if ids.len() < 3 {
            return;
        }
        self.push_undo();
        let mut items: Vec<(String, RectMm)> = {
            let page = self.current_page();
            ids.iter()
                .filter_map(|id| page.components.iter().find(|c| &c.id == id).map(|c| (id.clone(), c.rect)))
                .collect()
        };
        let key = |r: &RectMm| if horizontal { r.x } else { r.y };
        let size = |r: &RectMm| if horizontal { r.w } else { r.h };
        items.sort_by(|a, b| key(&a.1).partial_cmp(&key(&b.1)).unwrap_or(std::cmp::Ordering::Equal));

        let first = key(&items[0].1);
        let last_item = items.last().expect("three or more items").1;
        let last_end = key(&last_item) + size(&last_item);
        let total: f32 = items.iter().map(|(_, r)| size(r)).sum();
        let gap = ((last_end - first) - total) / (items.len() as f32 - 1.0);

        let mut pos = first;
        let page = self.current_page_mut();
        for (id, rect) in &items {
            if let Some(c) = page.components.iter_mut().find(|c| &c.id == id) {
                if horizontal {
                    c.rect.x = pos;
                } else {
                    c.rect.y = pos;
                }
            }
            pos += size(rect) + gap;
        }
        self.status_message = "Distributed the selection.".to_string();
        self.refresh_code_buffer();
    }

    /// Moves the selected components to the front (drawn last) or to the back (drawn first).
    pub fn z_order(&mut self, to_front: bool) {
        let ids = self.selected_ids();
        if ids.is_empty() {
            return;
        }
        self.push_undo();
        let page = self.current_page_mut();
        let (chosen, rest): (Vec<Component>, Vec<Component>) =
            std::mem::take(&mut page.components).into_iter().partition(|c| ids.contains(&c.id));
        page.components = if to_front {
            rest.into_iter().chain(chosen).collect()
        } else {
            chosen.into_iter().chain(rest).collect()
        };
        self.status_message = if to_front { "Brought to front.".into() } else { "Sent to back.".into() };
        self.refresh_code_buffer();
    }
}
