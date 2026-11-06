//! Picture components: decoded once into GPU textures and cached by the picture's content.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use eframe::egui::{self, ColorImage, Color32, Painter, Pos2, Rect, TextureHandle, TextureOptions};

/// Textures keyed by `key(bytes)`.
pub type ImageCache = HashMap<u64, TextureHandle>;

/// Cache key for a picture: a hash of its bytes.
pub fn key(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

/// Decodes and caches the pictures that are not in the cache yet. Bytes that are not an image
/// are skipped (the component then shows nothing).
pub fn ensure_loaded(ctx: &egui::Context, cache: &mut ImageCache, pictures: &[Vec<u8>]) {
    for bytes in pictures {
        let k = key(bytes);
        if cache.contains_key(&k) {
            continue;
        }
        let Ok(img) = image::load_from_memory(bytes) else { continue };
        let img = img.to_rgba8();
        let (w, h) = img.dimensions();
        let color = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
        let texture = ctx.load_texture(format!("picture_{k:x}"), color, TextureOptions::LINEAR);
        cache.insert(k, texture);
    }
}

/// Draws a cached picture stretched to `rect`.
pub fn paint(painter: &Painter, texture: &TextureHandle, rect: Rect) {
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}
