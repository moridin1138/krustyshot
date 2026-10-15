//! "Check for updates": compares this version with the latest GitHub release.
//! Runs when the user asks, and (if "Check for updates automatically" is on)
//! shortly after start-up and once a day. It sends a plain request to
//! GitHub's public API and nothing else.

use anyhow::{Context, Result};

pub const RELEASES_URL: &str = "https://github.com/moridin1138/rustshot/releases";
// Not `/releases/latest`: that skips pre-releases, and every 0.x release is one.
const RELEASES_API: &str = "https://api.github.com/repos/moridin1138/rustshot/releases?per_page=30";

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    UpToDate { latest: String },
    Available { version: String, url: String },
}

/// Ask GitHub for the newest published release. Blocking; call from a
/// background thread.
pub fn check(current: &str) -> Result<UpdateStatus> {
    let mut response = ureq::get(RELEASES_API)
        .header("User-Agent", concat!("Rustshot/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("contacting GitHub")?;
    let body = response.body_mut().read_to_string().context("reading GitHub's reply")?;
    let json: serde_json::Value = serde_json::from_str(&body).context("parsing GitHub's reply")?;
    let (tag, url) = newest(&json).context("no published releases found")?;
    Ok(compare(current, &tag, url))
}

/// The highest-versioned published (non-draft) release in a GitHub
/// releases list, as (tag, page URL).
fn newest(releases: &serde_json::Value) -> Option<(String, String)> {
    releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?.to_string();
            let url = r["html_url"].as_str().unwrap_or(RELEASES_URL).to_string();
            Some((tag, url))
        })
        .max_by_key(|(tag, _)| parse(tag))
}

fn compare(current: &str, latest_tag: &str, url: String) -> UpdateStatus {
    let latest = latest_tag.trim_start_matches('v').to_string();
    if parse(&latest) > parse(current) {
        UpdateStatus::Available { version: latest, url }
    } else {
        UpdateStatus::UpToDate { latest }
    }
}

/// `1.2.3` (or `1.2.3-beta.1`, suffix ignored) as comparable numbers.
fn parse(v: &str) -> (u64, u64, u64) {
    let core = v.trim_start_matches('v').split(['-', '+']).next().unwrap_or("");
    let mut parts = core.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Talks to GitHub; run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_check_against_github() {
        let status = check("0.0.1").unwrap_or_else(|e| panic!("{e:?}"));
        assert!(matches!(status, UpdateStatus::Available { .. }), "{status:?}");
    }

    #[test]
    fn newest_skips_drafts_and_sorts_by_version() {
        let list = serde_json::json!([
            {"tag_name": "v0.3.0", "draft": true, "html_url": "d"},
            {"tag_name": "v0.10.0", "draft": false, "html_url": "ten", "prerelease": true},
            {"tag_name": "v0.9.0", "draft": false, "html_url": "nine"}
        ]);
        assert_eq!(newest(&list), Some(("v0.10.0".into(), "ten".into())));
    }

    #[test]
    fn compares_numerically() {
        let u = String::new;
        assert_eq!(compare("0.2.0", "v0.10.0", u()), UpdateStatus::Available { version: "0.10.0".into(), url: u() });
        assert_eq!(compare("0.2.0", "v0.2.0", u()), UpdateStatus::UpToDate { latest: "0.2.0".into() });
        assert_eq!(compare("1.0.0", "v0.9.9", u()), UpdateStatus::UpToDate { latest: "0.9.9".into() });
        assert_eq!(parse("1.2.3-beta.1"), (1, 2, 3));
    }
}
