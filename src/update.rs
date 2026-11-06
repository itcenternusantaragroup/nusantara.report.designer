//! Checks GitHub for a newer release of the designer.
//!
//! The check runs on a background thread so the window never waits for the network. A release
//! is "newer" when it was published after this program was built (the build time is stored at
//! compile time by build.rs). Release tags are not compared directly, because the build number
//! in the tag does not follow the build counter shown in the About window.

use std::sync::mpsc::Sender;
use std::time::Duration;

const LATEST_API: &str =
    "https://api.github.com/repos/itcenternusantaragroup/nusantara.report.designer/releases/latest";
/// The releases page, shown when the API does not give a link.
pub const RELEASES_PAGE: &str = "https://github.com/itcenternusantaragroup/nusantara.report.designer/releases";

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub published_unix: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckResult {
    /// A release published after this build.
    Newer(Release),
    UpToDate,
    Failed(String),
}

/// Unix time (seconds) when this program was compiled.
pub fn build_unix() -> i64 {
    env!("APP_BUILD_UNIX").parse().unwrap_or(0)
}

/// Asks GitHub for the latest published release (drafts and pre-releases are not included).
pub fn fetch_latest() -> Result<Release, String> {
    let text = ureq::get(LATEST_API)
        .set("User-Agent", "NusantaraReportDesigner")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(10))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let tag = body["tag_name"].as_str().ok_or("no tag in the response")?.to_string();
    let published = body["published_at"].as_str().ok_or("no publish date in the response")?;
    let published_unix = chrono::DateTime::parse_from_rfc3339(published)
        .map_err(|e| e.to_string())?
        .timestamp();
    let url = body["html_url"].as_str().unwrap_or(RELEASES_PAGE).to_string();
    Ok(Release { tag, url, published_unix })
}

/// The comparison used by the UI: a release is newer when it came out after this build.
pub fn classify(latest: Release, built_unix: i64) -> CheckResult {
    if latest.published_unix > built_unix {
        CheckResult::Newer(latest)
    } else {
        CheckResult::UpToDate
    }
}

/// Runs a check in the background and sends the result to `tx`.
pub fn spawn_check(tx: Sender<CheckResult>) {
    std::thread::spawn(move || {
        let result = match fetch_latest() {
            Ok(latest) => classify(latest, build_unix()),
            Err(e) => CheckResult::Failed(e),
        };
        let _ = tx.send(result);
    });
}

/// Opens a link in the default browser.
pub fn open_url(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(published_unix: i64) -> Release {
        Release { tag: "v1.0.1.16".into(), url: RELEASES_PAGE.into(), published_unix }
    }

    #[test]
    fn newer_release_is_reported() {
        assert!(matches!(classify(release(200), 100), CheckResult::Newer(_)));
    }

    #[test]
    fn older_or_same_release_is_not_reported() {
        assert_eq!(classify(release(100), 200), CheckResult::UpToDate);
        assert_eq!(classify(release(100), 100), CheckResult::UpToDate);
    }

    #[test]
    fn build_time_is_known() {
        assert!(build_unix() > 1_700_000_000, "build.rs stores the compile time");
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Calls GitHub. Runs only when NUSA_LIVE_UPDATE_TEST is set, so normal test runs stay offline.
    #[test]
    fn latest_release_can_be_read() {
        if std::env::var("NUSA_LIVE_UPDATE_TEST").is_err() {
            return;
        }
        let r = fetch_latest().expect("GitHub answers");
        println!("tag={} published={} url={}", r.tag, r.published_unix, r.url);
        assert!(r.tag.starts_with('v'));
        assert!(r.published_unix > 1_600_000_000);
    }
}