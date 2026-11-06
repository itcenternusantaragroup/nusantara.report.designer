use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use uuid::Uuid;

use crate::model::*;
use super::designer::{ActiveTool, DragState, ReportDesignerApp, ResizeHandle};
use super::theme;

pub struct CanvasView;

impl CanvasView {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style())
                .fill(theme::DESK))
            .show(ctx, |ui| {
                if app.fit_to_window {
                    app.zoom = Self::fit_zoom(app, ui.available_size());
                }
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        Self::draw_canvas(app, ui);
                    });
            });
    }

    /// Largest zoom at which the whole current page fits in `avail`,
    /// leaving room for rulers and the drop shadow (same margins as `draw_canvas`).
    fn fit_zoom(app: &ReportDesignerApp, avail: Vec2) -> f32 {
        let (page_w_mm, page_h_mm) = app.current_page().effective_dimensions();
        let ruler_size = if app.show_rulers { 22.0 } else { 0.0 };
        let usable_w = avail.x - (ruler_size + 30.0) - 80.0;
        let usable_h = avail.y - (ruler_size + 20.0) - 80.0;
        let zx = usable_w / (page_w_mm * 3.78);
        let zy = usable_h / (page_h_mm * 3.78);
        zx.min(zy).clamp(0.3, 3.0)
    }

    fn draw_canvas(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        // Decode pictures that are not loaded yet, before anything else is borrowed.
        let missing: Vec<Vec<u8>> = app
            .current_page()
            .components
            .iter()
            .filter_map(|c| c.image_data.as_ref())
            .filter(|b| !app.images.contains_key(&super::images::key(b)))
            .cloned()
            .collect();
        super::images::ensure_loaded(ui.ctx(), &mut app.images, &missing);

        let zoom = app.zoom;
        let page = app.current_page();
        let (page_w_mm, page_h_mm) = page.effective_dimensions();

        // 1 mm = 3.78 px at 96 DPI
        let mm_to_px = 3.78 * zoom;
        let page_w_px = page_w_mm * mm_to_px;
        let page_h_px = page_h_mm * mm_to_px;

        let ruler_size = if app.show_rulers { 22.0 } else { 0.0 };
        // Centre the sheet horizontally when it is narrower than the viewport; never closer than the ruler.
        let centred_x = ((ui.available_width() - page_w_px) * 0.5).max(0.0);
        let min_x: f32 = ruler_size + 30.0;
        let offset = Pos2::new(min_x.max(centred_x), ruler_size + 20.0);

        let total_size = Vec2::new(page_w_px + offset.x + 80.0, page_h_px + offset.y + 80.0);
        let (response, painter) = ui.allocate_painter(total_size, egui::Sense::click_and_drag());

        // `offset` is relative to the canvas origin, not to the window.
        let page_rect = Rect::from_min_size(response.rect.min + offset.to_vec2(), Vec2::new(page_w_px, page_h_px));

        // 1. Page: a soft two-layer shadow so the sheet sits on the desk.
        for (grow, alpha) in [(3.0_f32, 10_u8), (1.0, 18)] {
            let shadow_rect = page_rect.translate(Vec2::new(0.0, 2.0)).expand(grow);
            painter.rect_filled(shadow_rect, 2.0, Color32::from_black_alpha(alpha));
        }
        painter.rect_filled(page_rect, 0.0, theme::PAPER);
        painter.rect_stroke(page_rect, 0.0, Stroke::new(1.0_f32, theme::RULE));

        // 2. Margins: a faint guide inside the page.
        let margin_rect = Rect::from_min_max(
            Pos2::new(page_rect.min.x + page.margins.left * mm_to_px, page_rect.min.y + page.margins.top * mm_to_px),
            Pos2::new(page_rect.max.x - page.margins.right * mm_to_px, page_rect.max.y - page.margins.bottom * mm_to_px),
        );
        painter.rect_stroke(margin_rect, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0xEC, 0xEF, 0xF3)));

        // 3. Grid dots
        if app.show_grid {
            let grid_step_px = app.grid_size_mm * mm_to_px;
            if grid_step_px >= 6.0 {
                let dot_color = Color32::from_rgb(0xDD, 0xE1, 0xE7);
                let mut x = page_rect.min.x + grid_step_px;
                while x < page_rect.max.x {
                    let mut y = page_rect.min.y + grid_step_px;
                    while y < page_rect.max.y {
                        painter.circle_filled(Pos2::new(x, y), 0.75, dot_color);
                        y += grid_step_px;
                    }
                    x += grid_step_px;
                }
            }
        }

        // 4. Draw Bands (Stimulsoft Style Colored Header Ribbons)
        for band in &app.current_page().bands {
            let b_top_px = page_rect.min.y + band.top * mm_to_px;
            let b_height_px = band.height * mm_to_px;
            let b_width_px = page.printable_width() * mm_to_px;
            let b_left_px = page_rect.min.x + page.margins.left * mm_to_px;

            let band_rect = Rect::from_min_size(Pos2::new(b_left_px, b_top_px), Vec2::new(b_width_px, b_height_px));

            let band_color_arr = band.band_type.default_color();
            let band_color = Color32::from_rgba_premultiplied(band_color_arr[0], band_color_arr[1], band_color_arr[2], 255);

            // Sleek Band Header Banner Strip
            let header_height = 16.0;
            let header_rect = Rect::from_min_size(Pos2::new(b_left_px, b_top_px - header_height), Vec2::new(b_width_px, header_height));
            painter.rect_filled(header_rect, 1.0, band_color);

            let label = if let Some(ds) = &band.data_source {
                format!("{}: {} [{ds}]", band.name, band.band_type.display_name())
            } else {
                format!("{}: {}", band.name, band.band_type.display_name())
            };

            painter.text(
                header_rect.left_center() + Vec2::new(6.0, 0.0),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );

            // Band body outline
            let is_band_sel = app.selected_band_id.as_deref() == Some(&band.id);
            if is_band_sel {
                painter.rect_filled(band_rect, 0.0, Color32::from_rgba_unmultiplied(band_color_arr[0], band_color_arr[1], band_color_arr[2], 20));
                painter.rect_stroke(band_rect, 0.0, Stroke::new(1.5_f32, band_color));
            } else {
                painter.rect_stroke(band_rect, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0xE6, 0xE9, 0xEE)));
            }

            // Band bottom edge resize indicator
            let bottom_edge_rect = Rect::from_min_size(
                Pos2::new(b_left_px, b_top_px + b_height_px - 2.0),
                Vec2::new(b_width_px, 3.0),
            );
            painter.rect_filled(bottom_edge_rect, 0.0, band_color);
        }

        // 5. Draw Components
        for comp in &app.current_page().components {
            let comp_x_px = page_rect.min.x + comp.rect.x * mm_to_px;
            let comp_y_px = page_rect.min.y + comp.rect.y * mm_to_px;
            let comp_w_px = comp.rect.w * mm_to_px;
            let comp_h_px = comp.rect.h * mm_to_px;

            let c_rect = Rect::from_min_size(Pos2::new(comp_x_px, comp_y_px), Vec2::new(comp_w_px, comp_h_px));

            // Background Fill
            if comp.bg_color[3] > 0 {
                let fill_c = Color32::from_rgba_premultiplied(comp.bg_color[0], comp.bg_color[1], comp.bg_color[2], comp.bg_color[3]);
                painter.rect_filled(c_rect, 0.0, fill_c);
            }

            // Borders
            if comp.border_sides.has_any() {
                let border_c = Color32::from_rgba_premultiplied(comp.border_color[0], comp.border_color[1], comp.border_color[2], comp.border_color[3]);
                let bw = (comp.border_width * mm_to_px).max(1.0);
                let stroke = Stroke::new(bw, border_c);

                if comp.border_sides.top {
                    painter.line_segment([c_rect.left_top(), c_rect.right_top()], stroke);
                }
                if comp.border_sides.bottom {
                    painter.line_segment([c_rect.left_bottom(), c_rect.right_bottom()], stroke);
                }
                if comp.border_sides.left {
                    painter.line_segment([c_rect.left_top(), c_rect.left_bottom()], stroke);
                }
                if comp.border_sides.right {
                    painter.line_segment([c_rect.right_top(), c_rect.right_bottom()], stroke);
                }
            }

            // Draw Content by Type
            match comp.comp_type {
                ComponentType::Text => {
                    super::richtext::draw(&painter, comp, &comp.text, c_rect, super::richtext::px_per_pt(zoom));
                }
                ComponentType::Barcode => {
                    let bc_rect = c_rect.shrink(2.0);
                    let mut bx = bc_rect.min.x;
                    let bar_w = 2.0 * zoom;
                    let mut toggle = false;
                    while bx < bc_rect.max.x {
                        if toggle {
                            painter.rect_filled(
                                Rect::from_min_max(Pos2::new(bx, bc_rect.min.y), Pos2::new(bx + bar_w, bc_rect.max.y - 11.0)),
                                0.0,
                                Color32::BLACK,
                            );
                        }
                        bx += bar_w + (if (bx as usize) % 3 == 0 { 1.5 * bar_w } else { 0.5 * bar_w });
                        toggle = !toggle;
                    }
                    painter.text(
                        bc_rect.center_bottom() + Vec2::new(0.0, -1.0),
                        egui::Align2::CENTER_BOTTOM,
                        &comp.text,
                        egui::FontId::monospace(9.0 * zoom),
                        Color32::BLACK,
                    );
                }
                ComponentType::Image => match comp.image_data.as_ref().and_then(|b| app.images.get(&super::images::key(b))) {
                    Some(texture) => super::images::paint(&painter, texture, c_rect),
                    None => {
                        painter.rect_stroke(c_rect, 0.0, Stroke::new(1.0_f32, theme::RULE));
                        painter.text(
                            c_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Image",
                            egui::FontId::proportional(11.0 * zoom),
                            Color32::GRAY,
                        );
                    }
                },
                ComponentType::Shape => {
                    let border_c = Color32::from_rgba_premultiplied(comp.border_color[0], comp.border_color[1], comp.border_color[2], comp.border_color[3]);
                    let fill_c = Color32::from_rgba_premultiplied(comp.bg_color[0], comp.bg_color[1], comp.bg_color[2], comp.bg_color[3]);
                    let bw = (comp.border_width * mm_to_px).max(1.0);
                    let stroke = Stroke::new(bw, border_c);

                    match comp.shape_kind {
                        ShapeKind::Rectangle => {
                            painter.rect(c_rect, 0.0, fill_c, stroke);
                        }
                        ShapeKind::RoundedRectangle => {
                            painter.rect(c_rect, 6.0 * zoom, fill_c, stroke);
                        }
                        ShapeKind::Ellipse => {
                            painter.circle(c_rect.center(), (c_rect.width() * 0.5).min(c_rect.height() * 0.5), fill_c, stroke);
                        }
                        ShapeKind::Triangle => {
                            let p1 = c_rect.center_top();
                            let p2 = c_rect.right_bottom();
                            let p3 = c_rect.left_bottom();
                            painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], fill_c, stroke));
                        }
                    }
                }
                ComponentType::Line => {
                    let border_c = Color32::from_rgba_premultiplied(comp.border_color[0], comp.border_color[1], comp.border_color[2], comp.border_color[3]);
                    let bw = (comp.border_width * mm_to_px).max(1.5);
                    // Draw along the longer side, so vertical lines (zero width) are visible.
                    let (a, b) = if c_rect.width() < c_rect.height() {
                        (c_rect.center_top(), c_rect.center_bottom())
                    } else {
                        (c_rect.left_center(), c_rect.right_center())
                    };
                    painter.line_segment([a, b], Stroke::new(bw, border_c));
                }
                ComponentType::CheckBox => {
                    let box_size = 14.0 * zoom;
                    let box_rect = Rect::from_min_size(c_rect.left_center() + Vec2::new(0.0, -box_size * 0.5), Vec2::splat(box_size));
                    painter.rect(box_rect, 1.0, Color32::WHITE, Stroke::new(1.0_f32, Color32::BLACK));
                    if comp.checked {
                        painter.text(box_rect.center(), egui::Align2::CENTER_CENTER, "✔", egui::FontId::proportional(12.0 * zoom), Color32::BLACK);
                    }
                    painter.text(
                        box_rect.right_center() + Vec2::new(4.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        &comp.text,
                        egui::FontId::proportional(10.0 * zoom),
                        Color32::BLACK,
                    );
                }
                ComponentType::Table => {
                    painter.rect_stroke(c_rect, 0.0, Stroke::new(1.0_f32, Color32::BLACK));
                    painter.text(c_rect.center(), egui::Align2::CENTER_CENTER, "Table Grid", egui::FontId::proportional(11.0 * zoom), Color32::GRAY);
                }
            }

            // Other selected components get an outline; the primary one gets handles below.
            if app.extra_selected.contains(&comp.id) {
                painter.rect_stroke(c_rect, 0.0, Stroke::new(1.5_f32, theme::ACCENT));
            }

            // Selection Bounding Box & 8 Grab Handles (Stimulsoft Sapphire Blue)
            if app.selected_component_id.as_deref() == Some(&comp.id) {
                let sel_color = theme::ACCENT;
                painter.rect_stroke(c_rect, 0.0, Stroke::new(1.5_f32, sel_color));

                let handle_size = 8.0;
                let handles = [
                    c_rect.left_top(),
                    c_rect.center_top(),
                    c_rect.right_top(),
                    c_rect.right_center(),
                    c_rect.right_bottom(),
                    c_rect.center_bottom(),
                    c_rect.left_bottom(),
                    c_rect.left_center(),
                ];

                for hp in handles {
                    let h_rect = Rect::from_center_size(hp, Vec2::splat(handle_size));
                    painter.rect_filled(h_rect, 0.0, Color32::WHITE);
                    painter.rect_stroke(h_rect, 0.0, Stroke::new(1.2_f32, sel_color));
                }
            }
        }

        // 6. Draw Rulers
        if app.show_rulers {
            Self::draw_rulers(app, &painter, page_rect, ruler_size, mm_to_px);
        }

        // 7. Interactive Mouse Events: Selection, Dragging, Resizing, Insertion
        // Rubber band while it is being dragged.
        if let DragState::Marquee { start } = app.drag_state {
            if let Some(now) = response.ctx.input(|i| i.pointer.interact_pos()) {
                let r = Rect::from_two_pos(Pos2::new(start.0, start.1), now);
                painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(0x1A, 0x73, 0xE8, 30));
                painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, theme::ACCENT));
            }
        }

        Self::handle_mouse_interactions(app, &response, page_rect, mm_to_px);
    }

    fn draw_rulers(app: &ReportDesignerApp, painter: &egui::Painter, page_rect: Rect, ruler_size: f32, mm_to_px: f32) {
        let ruler_bg = theme::CHROME;
        let tick_c = theme::MUTED;

        // Top Ruler
        let top_ruler_rect = Rect::from_min_max(
            Pos2::new(page_rect.min.x, page_rect.min.y - ruler_size),
            Pos2::new(page_rect.max.x, page_rect.min.y),
        );
        painter.rect_filled(top_ruler_rect, 0.0, ruler_bg);
        painter.line_segment([top_ruler_rect.left_bottom(), top_ruler_rect.right_bottom()], Stroke::new(1.0_f32, tick_c));

        let mut mm = 0.0;
        let page_w_mm = app.current_page().effective_dimensions().0;
        while mm <= page_w_mm {
            let x = page_rect.min.x + mm * mm_to_px;
            let is_cm = (mm % 10.0).abs() < 0.01;
            let tick_h = if is_cm { ruler_size * 0.6 } else { ruler_size * 0.3 };
            painter.line_segment([Pos2::new(x, page_rect.min.y - tick_h), Pos2::new(x, page_rect.min.y)], Stroke::new(1.0_f32, tick_c));

            if is_cm && mm > 0.0 {
                let cm_val = (mm / 10.0) as u32;
                painter.text(Pos2::new(x + 2.0, page_rect.min.y - ruler_size + 2.0), egui::Align2::LEFT_TOP, format!("{cm_val}"), egui::FontId::proportional(9.0), tick_c);
            }
            mm += 2.0;
        }

        // Left Ruler
        let left_ruler_rect = Rect::from_min_max(
            Pos2::new(page_rect.min.x - ruler_size, page_rect.min.y),
            Pos2::new(page_rect.min.x, page_rect.max.y),
        );
        painter.rect_filled(left_ruler_rect, 0.0, ruler_bg);
        painter.line_segment([left_ruler_rect.right_top(), left_ruler_rect.right_bottom()], Stroke::new(1.0_f32, tick_c));

        let mut mm_y = 0.0;
        let page_h_mm = app.current_page().effective_dimensions().1;
        while mm_y <= page_h_mm {
            let y = page_rect.min.y + mm_y * mm_to_px;
            let is_cm = (mm_y % 10.0).abs() < 0.01;
            let tick_w = if is_cm { ruler_size * 0.6 } else { ruler_size * 0.3 };
            painter.line_segment([Pos2::new(page_rect.min.x - tick_w, y), Pos2::new(page_rect.min.x, y)], Stroke::new(1.0_f32, tick_c));

            if is_cm && mm_y > 0.0 {
                let cm_val = (mm_y / 10.0) as u32;
                painter.text(Pos2::new(page_rect.min.x - ruler_size + 2.0, y + 2.0), egui::Align2::LEFT_TOP, format!("{cm_val}"), egui::FontId::proportional(9.0), tick_c);
            }
            mm_y += 2.0;
        }
    }

    fn handle_mouse_interactions(
        app: &mut ReportDesignerApp,
        response: &egui::Response,
        page_rect: Rect,
        mm_to_px: f32,
    ) {
        // Cursor feedback: resize arrows over a selected component's handles, move over a body.
        if let Some(hover) = response.hover_pos() {
            let over_handle = app
                .selected_component()
                .and_then(|c| handle_at(hover, c.rect, page_rect, mm_to_px).map(|h| handle_cursor(h)));
            let icon = over_handle.or_else(|| {
                let hover_mm = ((hover.x - page_rect.min.x) / mm_to_px, (hover.y - page_rect.min.y) / mm_to_px);
                let inside = app.current_page().components.iter().any(|c| c.rect.contains(hover_mm.0, hover_mm.1));
                inside.then_some(egui::CursorIcon::Move)
            });
            if let Some(icon) = icon {
                response.ctx.set_cursor_icon(icon);
            }
        }

        let mouse_pos = response.interact_pointer_pos();

        let mouse_mm = mouse_pos.map(|p| {
            (
                (p.x - page_rect.min.x) / mm_to_px,
                (p.y - page_rect.min.y) / mm_to_px,
            )
        });

        // 1. Tool Insertion
        if response.clicked() {
            if let Some((mx_mm, my_mm)) = mouse_mm {
                match app.active_tool.clone() {
                    ActiveTool::InsertComponent(comp_type) => {
                        app.push_undo();
                        let mut snap_x = mx_mm;
                        let mut snap_y = my_mm;
                        if app.snap_to_grid {
                            snap_x = (snap_x / app.grid_size_mm).round() * app.grid_size_mm;
                            snap_y = (snap_y / app.grid_size_mm).round() * app.grid_size_mm;
                        }

                        let mut comp = Component::default();
                        comp.comp_type = comp_type;
                        comp.rect = RectMm::new(snap_x, snap_y, 40.0, 10.0);
                        comp.name = format!("{}_{}", comp_type.display_name(), app.current_page().components.len() + 1);

                        for band in &app.current_page().bands {
                            if snap_y >= band.top && snap_y <= (band.top + band.height) {
                                comp.parent_band_id = Some(band.id.clone());
                                break;
                            }
                        }

                        let new_id = comp.id.clone();
                        app.current_page_mut().components.push(comp);
                        app.selected_component_id = Some(new_id);
                        app.selected_band_id = None;
                        app.active_tool = ActiveTool::Pointer;
                        app.status_message = format!("Created {} component.", comp_type.display_name());
                        app.refresh_code_buffer();
                        return;
                    }
                    ActiveTool::InsertBand(band_type) => {
                        app.push_undo();
                        let mut snap_y = my_mm;
                        if app.snap_to_grid {
                            snap_y = (snap_y / app.grid_size_mm).round() * app.grid_size_mm;
                        }

                        let band = Band {
                            id: Uuid::new_v4().to_string(),
                            name: format!("{}_{}", band_type.display_name().replace(' ', ""), app.current_page().bands.len() + 1),
                            band_type,
                            top: snap_y,
                            height: 15.0,
                            data_source: None,
                            group_by: None,
                        };

                        let new_id = band.id.clone();
                        app.current_page_mut().bands.push(band);
                        app.selected_band_id = Some(new_id);
                        app.selected_component_id = None;
                        app.active_tool = ActiveTool::Pointer;
                        app.status_message = format!("Added {} band.", band_type.display_name());
                        app.refresh_code_buffer();
                        return;
                    }
                    ActiveTool::Pointer => {
                        let mut clicked_comp_id = None;
                        for comp in app.current_page().components.iter().rev() {
                            if comp.rect.contains(mx_mm, my_mm) {
                                clicked_comp_id = Some(comp.id.clone());
                                break;
                            }
                        }

                        if let Some(c_id) = clicked_comp_id {
                            let additive = response.ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
                            app.select_component(&c_id, additive);
                        } else {
                            let mut clicked_band_id = None;
                            for band in &app.current_page().bands {
                                if my_mm >= (band.top - 5.0) && my_mm <= (band.top + band.height) {
                                    clicked_band_id = Some(band.id.clone());
                                    break;
                                }
                            }

                            if let Some(b_id) = clicked_band_id {
                                app.selected_band_id = Some(b_id);
                                app.selected_component_id = None;
                            } else {
                                app.selected_component_id = None;
                                app.selected_band_id = None;
                            }
                        }
                    }
                }
            }
        }

        // 2. Dragging & Resizing Interactions
        // Handles are checked in screen pixels, so they are the same size at every zoom and only
        // a press right on a handle starts a resize. A press anywhere else inside a component moves it.
        if response.drag_started() {
            // Start from where the button was pressed, not where the pointer was when egui
            // recognised the drag (a few pixels later), so the handle stays under the pointer.
            let press_origin = response.ctx.input(|i| i.pointer.press_origin()).or(response.interact_pointer_pos());
            if let Some(press) = press_origin {
                let mx_mm = (press.x - page_rect.min.x) / mm_to_px;
                let my_mm = (press.y - page_rect.min.y) / mm_to_px;
                if let Some(comp) = app.selected_component() {
                    let c_rect = comp.rect;
                    if let Some(h) = handle_at(press, c_rect, page_rect, mm_to_px) {
                        let comp_id = comp.id.clone();
                        app.push_undo();
                        app.drag_state = DragState::Resizing {
                            comp_id,
                            handle: h,
                            initial_rect: c_rect,
                            mouse_start_mm: (mx_mm, my_mm),
                        };
                        return;
                    }
                }

                let mut found_comp = None;
                for comp in app.current_page().components.iter().rev() {
                    if comp.rect.contains(mx_mm, my_mm) {
                        found_comp = Some((comp.id.clone(), comp.rect));
                        break;
                    }
                }

                if let Some((c_id, initial_rect)) = found_comp {
                    // Pressing a component that is not selected selects it alone; pressing one that is
                    // selected moves the whole selection.
                    if !app.selected_ids().contains(&c_id) {
                        app.select_component(&c_id, false);
                    }
                    app.push_undo();
                    app.selected_band_id = None;
                    let others: Vec<(String, RectMm)> = {
                        let page = app.current_page();
                        app.selected_ids()
                            .into_iter()
                            .filter(|id| *id != c_id)
                            .filter_map(|id| page.components.iter().find(|c| c.id == id).map(|c| (id, c.rect)))
                            .collect()
                    };
                    app.drag_state = DragState::Moving {
                        comp_id: c_id,
                        initial_rect,
                        mouse_start_mm: (mx_mm, my_mm),
                        others,
                    };
                    return;
                }

                let mut hit_band_resize = None;
                for band in &app.current_page().bands {
                    let bottom_y = band.top + band.height;
                    if (my_mm - bottom_y).abs() <= 3.0 {
                        hit_band_resize = Some((band.id.clone(), band.height));
                        break;
                    }
                }

                if hit_band_resize.is_none() {
                    // Nothing under the press: start a rubber-band selection.
                    app.drag_state = DragState::Marquee { start: (press.x, press.y) };
                    return;
                }

                if let Some((b_id, b_height)) = hit_band_resize {
                    app.push_undo();
                    app.drag_state = DragState::ResizingBand {
                        band_id: b_id,
                        initial_height: b_height,
                        mouse_start_y_mm: my_mm,
                    };
                    return;
                }
            }
        }

        if response.dragged() {
            if let Some((mx_mm, my_mm)) = mouse_mm {
                match app.drag_state.clone() {
                    DragState::Moving { comp_id, initial_rect, mouse_start_mm, others } => {
                        let dx = mx_mm - mouse_start_mm.0;
                        let dy = my_mm - mouse_start_mm.1;
                        let mut new_x = initial_rect.x + dx;
                        let mut new_y = initial_rect.y + dy;

                        if app.snap_to_grid {
                            new_x = magnetic(new_x, app.grid_size_mm);
                            new_y = magnetic(new_y, app.grid_size_mm);
                        }

                        if let Some(comp) = app.current_page_mut().components.iter_mut().find(|c| c.id == comp_id) {
                            comp.rect.x = new_x.max(0.0);
                            comp.rect.y = new_y.max(0.0);
                        }
                        // The other selected components keep their offset from the dragged one.
                        let (ddx, ddy) = (new_x.max(0.0) - initial_rect.x, new_y.max(0.0) - initial_rect.y);
                        for (oid, orect) in &others {
                            if let Some(c) = app.current_page_mut().components.iter_mut().find(|c| &c.id == oid) {
                                c.rect.x = (orect.x + ddx).max(0.0);
                                c.rect.y = (orect.y + ddy).max(0.0);
                            }
                        }
                    }
                    DragState::Resizing { comp_id, handle, initial_rect, mouse_start_mm } => {
                        let dx = mx_mm - mouse_start_mm.0;
                        let dy = my_mm - mouse_start_mm.1;

                        // Work on the four edges. Only the edges the handle controls move; the
                        // opposite edges stay exactly where they were.
                        let (mut left, mut top) = (initial_rect.x, initial_rect.y);
                        let (mut right, mut bottom) = (initial_rect.x + initial_rect.w, initial_rect.y + initial_rect.h);
                        let moves_left = matches!(handle, ResizeHandle::TopLeft | ResizeHandle::Left | ResizeHandle::BottomLeft);
                        let moves_right = matches!(handle, ResizeHandle::TopRight | ResizeHandle::Right | ResizeHandle::BottomRight);
                        let moves_top = matches!(handle, ResizeHandle::TopLeft | ResizeHandle::Top | ResizeHandle::TopRight);
                        let moves_bottom = matches!(handle, ResizeHandle::BottomLeft | ResizeHandle::Bottom | ResizeHandle::BottomRight);

                        if moves_left {
                            left += dx;
                        }
                        if moves_right {
                            right += dx;
                        }
                        if moves_top {
                            top += dy;
                        }
                        if moves_bottom {
                            bottom += dy;
                        }

                        if app.snap_to_grid {
                            let g = app.grid_size_mm;
                            if moves_left {
                                left = magnetic(left, g);
                            }
                            if moves_right {
                                right = magnetic(right, g);
                            }
                            if moves_top {
                                top = magnetic(top, g);
                            }
                            if moves_bottom {
                                bottom = magnetic(bottom, g);
                            }
                        }

                        // Keep the moving edge on the far side of the fixed edge, and not off the page.
                        if moves_left {
                            left = left.min(right - MIN_SIZE_MM).max(0.0);
                        }
                        if moves_right {
                            right = right.max(left + MIN_SIZE_MM);
                        }
                        if moves_top {
                            top = top.min(bottom - MIN_SIZE_MM).max(0.0);
                        }
                        if moves_bottom {
                            bottom = bottom.max(top + MIN_SIZE_MM);
                        }

                        if let Some(comp) = app.current_page_mut().components.iter_mut().find(|c| c.id == comp_id) {
                            comp.rect.x = left;
                            comp.rect.y = top;
                            comp.rect.w = right - left;
                            comp.rect.h = bottom - top;
                        }
                    }
                    DragState::ResizingBand { band_id, initial_height, mouse_start_y_mm } => {
                        let dy = my_mm - mouse_start_y_mm;
                        let mut new_h = (initial_height + dy).max(5.0);
                        if app.snap_to_grid {
                            new_h = (new_h / app.grid_size_mm).round() * app.grid_size_mm;
                        }
                        if let Some(band) = app.current_page_mut().bands.iter_mut().find(|b| b.id == band_id) {
                            band.height = new_h;
                        }
                    }
                    _ => {}
                }
            }
        }

        if response.drag_stopped() {
            if let DragState::Marquee { start } = app.drag_state.clone() {
                let end = response.ctx.input(|i| i.pointer.interact_pos()).unwrap_or(Pos2::new(start.0, start.1));
                let (x0, x1) = (start.0.min(end.x), start.0.max(end.x));
                let (y0, y1) = (start.1.min(end.y), start.1.max(end.y));
                let (lx0, lx1) = ((x0 - page_rect.min.x) / mm_to_px, (x1 - page_rect.min.x) / mm_to_px);
                let (ly0, ly1) = ((y0 - page_rect.min.y) / mm_to_px, (y1 - page_rect.min.y) / mm_to_px);
                let ids: Vec<String> = app
                    .current_page()
                    .components
                    .iter()
                    .filter(|c| c.rect.x < lx1 && c.rect.x + c.rect.w > lx0 && c.rect.y < ly1 && c.rect.y + c.rect.h > ly0)
                    .map(|c| c.id.clone())
                    .collect();
                if ids.is_empty() {
                    app.clear_component_selection();
                } else {
                    app.select_many(ids);
                }
            }
            app.drag_state = DragState::None;
            app.refresh_code_buffer();
        }
    }
}

/// Radius, in screen pixels, around a resize handle that still counts as grabbing it.
const HANDLE_RADIUS_PX: f32 = 7.0;

/// The resize handle under `pos` for component rect `c`, if any. Tested in screen pixels.
fn handle_at(pos: Pos2, c: RectMm, page_rect: Rect, mm_to_px: f32) -> Option<ResizeHandle> {
    let to_px = |x: f32, y: f32| Pos2::new(page_rect.min.x + x * mm_to_px, page_rect.min.y + y * mm_to_px);
    let (cx, cy) = (c.x + c.w * 0.5, c.y + c.h * 0.5);
    let handles = [
        (ResizeHandle::TopLeft, to_px(c.x, c.y)),
        (ResizeHandle::Top, to_px(cx, c.y)),
        (ResizeHandle::TopRight, to_px(c.x + c.w, c.y)),
        (ResizeHandle::Right, to_px(c.x + c.w, cy)),
        (ResizeHandle::BottomRight, to_px(c.x + c.w, c.y + c.h)),
        (ResizeHandle::Bottom, to_px(cx, c.y + c.h)),
        (ResizeHandle::BottomLeft, to_px(c.x, c.y + c.h)),
        (ResizeHandle::Left, to_px(c.x, cy)),
    ];
    handles
        .into_iter()
        .find(|(_, p)| p.distance(pos) <= HANDLE_RADIUS_PX)
        .map(|(h, _)| h)
}

/// Mouse cursor that matches the direction a handle resizes in.
fn handle_cursor(h: ResizeHandle) -> egui::CursorIcon {
    match h {
        ResizeHandle::TopLeft | ResizeHandle::BottomRight => egui::CursorIcon::ResizeNwSe,
        ResizeHandle::TopRight | ResizeHandle::BottomLeft => egui::CursorIcon::ResizeNeSw,
        ResizeHandle::Top | ResizeHandle::Bottom => egui::CursorIcon::ResizeVertical,
        ResizeHandle::Left | ResizeHandle::Right => egui::CursorIcon::ResizeHorizontal,
    }
}

/// Smallest width or height a component can be resized to, in mm.
const MIN_SIZE_MM: f32 = 2.0;

/// Snaps `v` to the grid only when it is close to a grid line (within 30% of the grid step).
/// Far from a line the value is kept as it is, so dragging stays smooth.
fn magnetic(v: f32, grid: f32) -> f32 {
    let nearest = (v / grid).round() * grid;
    if (nearest - v).abs() <= grid * 0.3 { nearest } else { v }
}
