//! Thunderstore API client.
//!
//! Mirrors `_ts_resolve_download_url` and `_ts_resolve_latest_name` from `odin.sh`.

use crate::error::{Error, Result};
use reqwest::Client;
use serde::Deserialize;

const API_BASE: &str = "https://thunderstore.io/api/experimental/package";
const CDN_BASE: &str = "https://gcdn.thunderstore.io/live/repository/packages";

#[derive(Debug, Deserialize)]
struct LatestVersion {
    pub version_number: String,
    pub download_url: String,
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
    pub latest: LatestVersion,
    pub community_listings: Option<Vec<CommunityListing>>,
}

#[derive(Debug, Deserialize)]
struct CommunityListing {
    pub categories: Vec<String>,
}

/// Resolved mod information from Thunderstore API.
#[derive(Debug, Clone)]
pub struct ResolvedMod {
    /// Full name with resolved version: "Author-Mod-X.Y.Z"
    pub full_name: String,
    /// Direct download URL for the zip.
    pub download_url: String,
}

/// Classification of a mod based on Thunderstore categories.
#[derive(Debug, Clone, PartialEq)]
pub enum ModCategory {
    ServerOnly,
    ClientOnly,
    Both,
    Unknown,
}

/// Resolve the latest version and download URL for a mod.
///
/// Falls back to the static CDN URL using `entry` if the API call fails.
pub async fn resolve_mod(client: &Client, namespace: &str, name: &str, entry: &str) -> ResolvedMod {
    match fetch_package(client, namespace, name).await {
        Ok(pkg) => ResolvedMod {
            full_name: format!("{namespace}-{name}-{}", pkg.latest.version_number),
            download_url: pkg.latest.download_url,
        },
        Err(_) => ResolvedMod {
            full_name: entry.to_string(),
            download_url: format!("{CDN_BASE}/{entry}.zip"),
        },
    }
}

/// Classify a mod as server-only / client-only / both / unknown.
pub async fn classify_mod(client: &Client, namespace: &str, name: &str) -> Result<ModCategory> {
    let pkg = fetch_package(client, namespace, name).await?;

    let categories: Vec<String> = pkg
        .community_listings
        .unwrap_or_default()
        .into_iter()
        .flat_map(|l| l.categories)
        .map(|c| c.to_lowercase())
        .collect();

    if categories.is_empty() {
        return Ok(ModCategory::Unknown);
    }

    let is_server = categories.iter().any(|c| c == "server-side");
    let is_client = categories.iter().any(|c| c == "client-side");

    Ok(match (is_server, is_client) {
        (true, true) => ModCategory::Both,
        (true, false) => ModCategory::ServerOnly,
        (false, true) => ModCategory::ClientOnly,
        _ => ModCategory::Unknown,
    })
}

async fn fetch_package(client: &Client, namespace: &str, name: &str) -> Result<PackageInfo> {
    let url = format!("{API_BASE}/{namespace}/{name}/");
    let resp = client
        .get(&url)
        .header("accept", "application/json")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(Error::network(format!(
            "HTTP {} for {namespace}/{name}",
            resp.status()
        )));
    }

    resp.json::<PackageInfo>()
        .await
        .map_err(|e| Error::network(e.to_string()))
}
