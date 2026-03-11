use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/bit5hift/sshm-rs/releases/latest";
const CACHE_TTL_SECS: i64 = 86_400; // 24 hours

pub struct UpdateInfo {
    pub latest_version: String,
}

#[derive(Serialize, Deserialize)]
struct UpdateCache {
    last_check: chrono::DateTime<chrono::Utc>,
    latest_version: String,
}

fn cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("sshm-rs").join("update-check.json"))
}

fn read_cache() -> Option<UpdateCache> {
    let path = cache_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_cache(version: &str) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let cache = UpdateCache {
        last_check: chrono::Utc::now(),
        latest_version: version.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = crate::config::write_private(&path, &json);
    }
}

fn fetch_latest_version() -> Option<String> {
    let current = env!("CARGO_PKG_VERSION");
    let user_agent = format!("sshm-rs/{current}");
    let timeout = std::time::Duration::from_secs(5);

    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(timeout))
            .build(),
    );

    let response = agent
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", &user_agent)
        .call()
        .ok()?;

    let mut resp_body = response.into_body();
    let body: serde_json::Value = resp_body.read_json().ok()?;
    let tag = body["tag_name"].as_str()?;
    let version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    Some(version)
}

/// Pure version comparison: returns true when `latest` is a valid semver
/// strictly greater than `current`. Returns false if either string cannot
/// be parsed as semver.
#[cfg(test)]
fn should_update(current: &str, latest: &str) -> bool {
    let Ok(cur) = semver::Version::parse(current) else {
        return false;
    };
    let Ok(lat) = semver::Version::parse(latest) else {
        return false;
    };
    lat > cur
}

pub fn check_for_update() -> Option<UpdateInfo> {
    if std::env::var("SSHM_NO_UPDATE_CHECK").as_deref() == Ok("1") {
        return None;
    }

    let current_str = env!("CARGO_PKG_VERSION");
    let current = semver::Version::parse(current_str).ok()?;

    let latest_str = if let Some(cache) = read_cache() {
        let age = chrono::Utc::now()
            .signed_duration_since(cache.last_check)
            .num_seconds();
        if age < CACHE_TTL_SECS {
            cache.latest_version
        } else {
            let fetched = fetch_latest_version()?;
            write_cache(&fetched);
            fetched
        }
    } else {
        let fetched = fetch_latest_version()?;
        write_cache(&fetched);
        fetched
    };

    let latest = semver::Version::parse(&latest_str).ok()?;

    if latest > current {
        Some(UpdateInfo {
            latest_version: latest_str,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{should_update, UpdateCache, UpdateInfo};
    use chrono::Utc;

    // --- should_update: version comparison logic ---

    #[test]
    fn update_available_when_latest_is_greater() {
        assert!(should_update("0.1.0", "0.2.0"));
    }

    #[test]
    fn no_update_when_current_is_greater() {
        assert!(!should_update("0.2.0", "0.1.0"));
    }

    #[test]
    fn no_update_when_versions_are_equal() {
        assert!(!should_update("0.1.0", "0.1.0"));
    }

    #[test]
    fn no_update_on_invalid_current_version() {
        // Unparseable current → conservative: do not claim an update
        assert!(!should_update("not-a-version", "0.2.0"));
    }

    #[test]
    fn no_update_on_invalid_latest_version() {
        // Unparseable latest → conservative: do not claim an update
        assert!(!should_update("0.1.0", "not-a-version"));
    }

    #[test]
    fn no_update_when_both_versions_invalid() {
        assert!(!should_update("bad", "also-bad"));
    }

    #[test]
    fn update_available_for_patch_bump() {
        assert!(should_update("1.0.0", "1.0.1"));
    }

    #[test]
    fn update_available_for_major_bump() {
        assert!(should_update("1.9.9", "2.0.0"));
    }

    #[test]
    fn no_update_for_pre_release_downgrade() {
        // 1.0.0-alpha < 1.0.0, so latest is NOT greater
        assert!(!should_update("1.0.0", "1.0.0-alpha"));
    }

    // --- UpdateInfo struct construction ---

    #[test]
    fn update_info_stores_version_string() {
        let info = UpdateInfo {
            latest_version: "1.2.3".to_string(),
        };
        assert_eq!(info.latest_version, "1.2.3");
    }

    // --- Cache round-trip via temp file ---

    #[test]
    fn cache_serializes_and_deserializes() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();

        let version = "2.0.0";
        let cache_out = UpdateCache {
            last_check: Utc::now(),
            latest_version: version.to_string(),
        };

        let json = serde_json::to_string(&cache_out).expect("serialize");
        std::fs::write(&path, &json).expect("write cache");

        let data = std::fs::read_to_string(&path).expect("read cache");
        let cache_in: UpdateCache = serde_json::from_str(&data).expect("deserialize");

        assert_eq!(cache_in.latest_version, version);
    }

    #[test]
    fn cache_deserialization_fails_on_corrupt_data() {
        let result: Result<UpdateCache, _> = serde_json::from_str("not json at all");
        assert!(result.is_err());
    }
}
