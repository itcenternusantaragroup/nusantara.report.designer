// Build number stamped at compile time, in the form YYYY.MM.DD.NN (local date, NN = builds that day).
// NN counts up by 1 on every compile and restarts at 1 when the date changes.
// State is kept in build_number.txt as "YYYY.MM.DD NN".
use std::fs;
use std::path::Path;

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
}
