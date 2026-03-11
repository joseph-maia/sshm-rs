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
