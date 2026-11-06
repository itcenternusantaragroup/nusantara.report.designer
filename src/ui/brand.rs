//! Brand assets: the application icon and logo, loaded from the WebP files in `assets/`.
//! Also the startup splash that shows the logo for a moment.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Id, Order, Rect, Vec2};
use image::{imageops::FilterType, RgbaImage};

use super::theme::{MUTED, PAPER};

pub static ICON_WEBP: &[u8] = include_bytes!("../../assets/Nusantara Report Designer Icon.webp");
pub static LOGO_WEBP: &[u8] = include_bytes!("../../assets/Nusantara Report Designer Logo.webp");

const SPLASH_TIME: Duration = Duration::from_millis(2200);

/// Textures created once at startup and reused by the title bar, the splash and About.
#[derive(Clone)]
pub struct Brand {
    pub icon: egui::TextureHandle,
    pub logo: egui::TextureHandle,
}

impl Brand {
    pub fn load(ctx: &egui::Context) -> Option<Self> {
        let mut icon = decode(ICON_WEBP)?;
        knock_out_background(&mut icon);
        let mut logo = decode(LOGO_WEBP)?;
        knock_out_background(&mut logo);
        Some(Self {
            icon: texture(ctx, "brand_icon", &icon, 256),
            logo: texture(ctx, "brand_logo", &logo, 1024),
        })
    }
}

/// Window icon for the operating system (taskbar, title bar).
pub fn window_icon() -> Option<egui::IconData> {
    let mut img = decode(ICON_WEBP)?;
    knock_out_background(&mut img);
    let img = image::imageops::resize(&img, 256, 256, FilterType::Lanczos3);
    let (width, height) = img.dimensions();
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}

fn decode(bytes: &[u8]) -> Option<RgbaImage> {
    image::load_from_memory_with_format(bytes, image::ImageFormat::WebP)
        .ok()
        .map(|img| img.to_rgba8())
}

/// Makes the white background around the icon transparent. White areas enclosed by the
/// artwork (the document page) are kept, because they are not reachable from the border.
fn knock_out_background(img: &mut RgbaImage) {
    let (w, h) = img.dimensions();
    let is_background = |p: &image::Rgba<u8>| p[0] >= 235 && p[1] >= 235 && p[2] >= 235;
    let mut seen = vec![false; (w * h) as usize];
    let mut queue = VecDeque::new();
    for x in 0..w {
        queue.push_back((x, 0));
        queue.push_back((x, h - 1));
    }
    for y in 0..h {
        queue.push_back((0, y));
        queue.push_back((w - 1, y));
    }

    while let Some((x, y)) = queue.pop_front() {
        let idx = (y * w + x) as usize;
        if seen[idx] {
            continue;
        }
        seen[idx] = true;
        if !is_background(img.get_pixel(x, y)) {
            continue;
        }
        img.get_pixel_mut(x, y)[3] = 0;
        if x > 0 {
            queue.push_back((x - 1, y));
        }
        if x + 1 < w {
            queue.push_back((x + 1, y));
        }
        if y > 0 {
            queue.push_back((x, y - 1));
        }
        if y + 1 < h {
            queue.push_back((x, y + 1));
        }
    }
}

fn texture(ctx: &egui::Context, name: &str, img: &RgbaImage, max_side: u32) -> egui::TextureHandle {
    let (w, h) = img.dimensions();
    let scaled = if w.max(h) > max_side {
        let s = max_side as f32 / w.max(h) as f32;
        let nw = ((w as f32 * s).round() as u32).max(1);
        let nh = ((h as f32 * s).round() as u32).max(1);
        image::imageops::resize(img, nw, nh, FilterType::Lanczos3)
    } else {
        img.clone()
    };
    let (w, h) = scaled.dimensions();
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], scaled.as_raw());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
}

/// Shows the splash for a short time after start-up. Returns true while it is visible.
/// Any click closes it early.
pub fn show_splash(ctx: &egui::Context, brand: &Brand, started: Instant) -> bool {
    let elapsed = started.elapsed();
    if elapsed >= SPLASH_TIME || ctx.input(|i| i.pointer.any_click()) {
        return false;
    }

    // Dim the window behind the splash.
    let screen: Rect = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("splash_dim")))
        .rect_filled(screen, 0.0, Color32::from_black_alpha(70));

    let logo_w = 380.0_f32;
    let size = brand.logo.size_vec2();
    let logo_h = logo_w * size.y / size.x;

    egui::Area::new(Id::new("splash"))
        .order(Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(PAPER)
                .rounding(14.0)
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 6.0),
                    blur: 28.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(70),
                })
                .inner_margin(egui::Margin::symmetric(36.0, 28.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Image::new(&brand.logo).fit_to_exact_size(Vec2::new(logo_w, logo_h)));
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Version {} · build {}",
                                env!("CARGO_PKG_VERSION"),
                                env!("APP_BUILD")
                            ))
                            .size(12.0)
                            .color(MUTED),
                        );
                    });
                });
        });

    ctx.request_repaint();
    true
}
