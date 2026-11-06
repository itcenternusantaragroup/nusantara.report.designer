// Build-time work:
// 1. Build number stamped at compile time, in the form YYYY.MM.DD.NN (local date, NN = builds that day).
//    NN counts up by 1 on every compile and restarts at 1 when the date changes.
//    State is kept in build_number.txt as "YYYY.MM.DD NN".
// 2. Windows only: the application icon is converted from assets/ to .ico and embedded in the
//    executable, so Explorer, the taskbar and shortcuts show it. Version info is added too.
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use image::ImageEncoder;

fn main() {
    let today = chrono::Local::now().format("%Y.%m.%d").to_string();

    let state = Path::new("build_number.txt");
    let (last_date, last_n) = fs::read_to_string(state)
        .ok()
        .and_then(|s| {
            let mut parts = s.split_whitespace();
            let date = parts.next()?.to_string();
            let n = parts.next()?.parse::<u32>().ok()?;
            Some((date, n))
        })
        .unwrap_or_default();

    let n = if last_date == today { last_n + 1 } else { 1 };
    let _ = fs::write(state, format!("{today} {n}\n"));

    println!("cargo:rustc-env=APP_BUILD={today}.{n}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=assets");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_resources(&format!("{today}.{n}"));
    }
}

/// Embeds the icon and version info into the Windows executable.
/// Uses windres (MinGW, already part of the GNU toolchain) and links the .res file into the binary.
fn embed_windows_resources(build: &str) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let ico_path = out_dir.join("app_icon.ico");
    write_ico("assets/Nusantara Report Designer Icon.webp", &ico_path);

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let build_field = build.rsplit('.').next().unwrap_or("0");
    let file_version = format!("{version}.{build_field}");
    let numeric = format!("{},{},{},{}", num(&version, 0), num(&version, 1), num(&version, 2), build_field);
    let icon = ico_path.to_string_lossy().replace('\\', "/");

    let rc = format!(
        "1 ICON \"{icon}\"\n\
         1 VERSIONINFO\n\
         FILEVERSION {numeric}\n\
         PRODUCTVERSION {numeric}\n\
         BEGIN\n\
         BLOCK \"StringFileInfo\"\n\
         BEGIN\n\
         BLOCK \"040904B0\"\n\
         BEGIN\n\
         VALUE \"FileDescription\", \"Nusantara Report Designer\0\"\n\
         VALUE \"ProductName\", \"Nusantara Report Designer\0\"\n\
         VALUE \"FileVersion\", \"{file_version}\0\"\n\
         VALUE \"ProductVersion\", \"{version} (build {build})\0\"\n\
         VALUE \"InternalName\", \"nusantara_report_designer\0\"\n\
         VALUE \"OriginalFilename\", \"nusantara_report_designer.exe\0\"\n\
         VALUE \"LegalCopyright\", \"(c) 2026. Licensed under GPL-3.0-or-later.\0\"\n\
         END\n\
         END\n\
         BLOCK \"VarFileInfo\"\n\
         BEGIN\n\
         VALUE \"Translation\", 0x409, 1200\n\
         END\n\
         END\n"
    );
    let rc_path = out_dir.join("app.rc");
    let res_path = out_dir.join("app.res");
    fs::write(&rc_path, rc).expect("write rc");

    let status = std::process::Command::new("windres")
        .arg("--input")
        .arg(&rc_path)
        .arg("--output")
        .arg(&res_path)
        .arg("--output-format=coff")
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-link-arg-bins={}", res_path.display());
        }
        _ => println!("cargo:warning=windres failed; the executable icon was not embedded"),
    }
}

/// Numeric part `index` of a dotted version such as "1.0.1" (0 when missing).
fn num(version: &str, index: usize) -> u32 {
    version.split('.').nth(index).and_then(|p| p.parse().ok()).unwrap_or(0)
}

/// Reads a WebP image, makes its white background transparent, and writes a 256 x 256 .ico.
fn write_ico(source: &str, target: &Path) {
    let bytes = fs::read(source).expect("read icon");
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP)
        .expect("decode icon")
        .to_rgba8();
    let mut img = img;
    knock_out_background(&mut img);
    let img = image::imageops::resize(&img, 256, 256, image::imageops::FilterType::Lanczos3);

    let file = File::create(target).expect("create ico");
    image::codecs::ico::IcoEncoder::new(file)
        .write_image(img.as_raw(), 256, 256, image::ExtendedColorType::Rgba8)
        .expect("write ico");
}

/// Same rule as the runtime window icon: white pixels connected to the border become transparent.
fn knock_out_background(img: &mut image::RgbaImage) {
    use std::collections::VecDeque;
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
