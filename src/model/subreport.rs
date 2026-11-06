//! Subreports: another report (`.mrt` file) printed inside a report.
//!
//! The path is kept as written in the component. A relative path is taken from the folder of the
//! report that contains the subreport. Loaded reports are cached per file and reloaded when the
//! file changes, because the page is laid out again on every redraw.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use super::mrt::MrtHandler;
use super::report::Report;

type Cache = Mutex<HashMap<PathBuf, (SystemTime, Arc<Report>)>>;
static CACHE: OnceLock<Cache> = OnceLock::new();

/// The file a subreport path points to, given the file of the report that contains it.
pub fn resolve(path: &str, base_report: Option<&str>) -> PathBuf {
    let p = Path::new(path.trim());
    if p.is_absolute() {
        return p.to_path_buf();
    }
    match base_report.and_then(|b| Path::new(b).parent()) {
        Some(dir) => dir.join(p),
        None => p.to_path_buf(),
    }
}

/// The subreport at `path`, or None when the path is empty, the file is missing or unreadable.
pub fn load(path: &str, base_report: Option<&str>) -> Option<Arc<Report>> {
    if path.trim().is_empty() {
        return None;
    }
    let full = resolve(path, base_report);
    let modified = std::fs::metadata(&full).ok()?.modified().ok()?;
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().ok()?;
    if let Some((time, report)) = map.get(&full) {
        if *time == modified {
            return Some(report.clone());
        }
    }
    let report = Arc::new(MrtHandler::load_from_file(&full).ok()?);
    map.insert(full, (modified, report.clone()));
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_taken_from_the_parent_report_folder() {
        assert_eq!(resolve("sub.mrt", Some("C:\\reports\\main.mrt")), PathBuf::from("C:\\reports").join("sub.mrt"));
        assert_eq!(resolve("sub.mrt", None), PathBuf::from("sub.mrt"));
        assert_eq!(resolve("D:\\x\\sub.mrt", Some("C:\\reports\\main.mrt")), PathBuf::from("D:\\x\\sub.mrt"));
    }

    #[test]
    fn missing_file_loads_as_none() {
        assert!(load("", None).is_none());
        assert!(load("no-such-file-xyz.mrt", None).is_none());
    }
}
