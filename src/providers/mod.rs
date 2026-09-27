//! Normalized modpack data shared by the Modrinth and CurseForge clients.

pub mod curseforge;
pub mod modrinth;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use shared::{State, models::Pagination, response::DisplayError};
use std::{cmp::Ordering, sync::LazyLock, time::Duration};
use utoipa::ToSchema;

use crate::settings::ExtensionSettingsData;

/// Seconds external listings (search, projects, versions) are cached for.
pub const CACHE_TTL: u64 = 300;
/// Seconds rarely changing data (categories, game versions) is cached for.
pub const LONG_CACHE_TTL: u64 = 6 * 60 * 60;

pub static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!(
            "Caloptreyx/MC-Modpack-Installer/",
            env!("CARGO_PKG_VERSION"),
            " (Calagopus extension dev.caloptreyx.modpacks)"
        ))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("failed to build modpack http client")
});

pub fn user_error(message: impl Into<String>, status: StatusCode) -> anyhow::Error {
    DisplayError::new(message.into()).with_status(status).into()
}

/// Sends a request to a modpack platform and decodes its JSON body, turning
/// failures into user-visible errors.
pub async fn fetch_json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
    platform: &str,
) -> Result<T, anyhow::Error> {
    let response = request.send().await.map_err(|err| {
        user_error(
            format!("{platform} is unreachable: {err}"),
            StatusCode::BAD_GATEWAY,
        )
    })?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(user_error(
            format!("not found on {platform}"),
            StatusCode::NOT_FOUND,
        ));
    }
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(user_error(
            format!("{platform} rejected the request (HTTP {status}), check the configured API key"),
            StatusCode::BAD_GATEWAY,
        ));
    }
    if !status.is_success() {
        return Err(user_error(
            format!("{platform} returned HTTP {status}"),
            StatusCode::BAD_GATEWAY,
        ));
    }

    response.json::<T>().await.map_err(|err| {
        user_error(
            format!("unexpected response from {platform}: {err}"),
            StatusCode::BAD_GATEWAY,
        )
    })
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Modrinth,
    Curseforge,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modrinth => "modrinth",
            Self::Curseforge => "curseforge",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "modrinth" => Some(Self::Modrinth),
            "curseforge" => Some(Self::Curseforge),
            _ => None,
        }
    }
}

/// Resolves which platform clients are usable with the current settings.
pub enum Client {
    Modrinth,
    Curseforge(String),
}

impl Client {
    pub fn new(settings: &ExtensionSettingsData, provider: Provider) -> Result<Self, anyhow::Error> {
        match provider {
            Provider::Modrinth if settings.modrinth_enabled => Ok(Self::Modrinth),
            Provider::Curseforge => match settings.curseforge_key() {
                Some(key) => Ok(Self::Curseforge(key.to_string())),
                None => Err(user_error(
                    "CurseForge is not available, ask an administrator to configure a CurseForge API key",
                    StatusCode::SERVICE_UNAVAILABLE,
                )),
            },
            Provider::Modrinth => Err(user_error(
                "Modrinth has been disabled by an administrator",
                StatusCode::SERVICE_UNAVAILABLE,
            )),
        }
    }

    pub async fn search(
        &self,
        state: &State,
        query: &SearchQuery,
    ) -> Result<Pagination<ModpackSummary>, anyhow::Error> {
        match self {
            Self::Modrinth => modrinth::search(state, query).await,
            Self::Curseforge(key) => curseforge::search(state, key, query).await,
        }
    }

    pub async fn filters(&self, state: &State) -> Result<Filters, anyhow::Error> {
        match self {
            Self::Modrinth => modrinth::filters(state).await,
            Self::Curseforge(key) => curseforge::filters(state, key).await,
        }
    }

    pub async fn project(&self, state: &State, id: &str) -> Result<ModpackDetails, anyhow::Error> {
        match self {
            Self::Modrinth => modrinth::project(state, id).await,
            Self::Curseforge(key) => curseforge::project(state, key, id).await,
        }
    }

    /// All versions of a project, newest first.
    pub async fn versions(
        &self,
        state: &State,
        project: &ModpackDetails,
    ) -> Result<Vec<ModpackVersion>, anyhow::Error> {
        match self {
            Self::Modrinth => modrinth::versions(state, &project.id).await,
            Self::Curseforge(key) => curseforge::versions(state, key, &project.id).await,
        }
    }

    pub async fn version(
        &self,
        state: &State,
        project: &ModpackDetails,
        version_id: &str,
    ) -> Result<VersionDetails, anyhow::Error> {
        match self {
            Self::Modrinth => modrinth::version(state, &project.id, version_id).await,
            Self::Curseforge(key) => curseforge::version(state, key, &project.id, version_id).await,
        }
    }

    /// The pack archive of a version, validated to belong to `project`.
    pub async fn pack_file(
        &self,
        project: &ModpackDetails,
        version_id: &str,
    ) -> Result<PackFile, anyhow::Error> {
        match self {
            Self::Modrinth => modrinth::pack_file(&project.id, version_id).await,
            Self::Curseforge(key) => curseforge::pack_file(key, &project.id, version_id).await,
        }
    }

    pub fn curseforge_key(&self) -> Option<&str> {
        match self {
            Self::Modrinth => None,
            Self::Curseforge(key) => Some(key),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Fabric,
    Forge,
    Neoforge,
    Quilt,
}

impl Loader {
    pub const ALL: [Loader; 4] = [Self::Fabric, Self::Forge, Self::Neoforge, Self::Quilt];

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "fabric" | "fabric-loader" => Some(Self::Fabric),
            "forge" => Some(Self::Forge),
            "neoforge" => Some(Self::Neoforge),
            "quilt" | "quilt-loader" => Some(Self::Quilt),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fabric => "fabric",
            Self::Forge => "forge",
            Self::Neoforge => "neoforge",
            Self::Quilt => "quilt",
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    Release,
    Beta,
    Alpha,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortMode {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TextFormat {
    Markdown,
    Html,
}

pub struct SearchQuery {
    pub query: Option<String>,
    pub loader: Option<Loader>,
    pub game_version: Option<String>,
    pub category: Option<String>,
    pub sort: SortMode,
    pub page: i64,
    pub per_page: i64,
    pub hide_client_only: bool,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ModpackSummary {
    pub provider: Provider,
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: i64,
    pub follows: i64,
    pub categories: Vec<String>,
    pub loaders: Vec<Loader>,
    /// Supported Minecraft releases, newest first.
    pub game_versions: Vec<String>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    /// Modrinth marks the pack as unsupported on servers.
    pub client_only: bool,
    pub url: String,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct GalleryImage {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ProjectLink {
    /// `website`, `source`, `issues`, `wiki`, `discord` or `donation`.
    pub kind: String,
    pub url: String,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ModpackDetails {
    pub provider: Provider,
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub authors: Vec<String>,
    pub icon_url: Option<String>,
    pub downloads: i64,
    pub follows: i64,
    pub categories: Vec<String>,
    pub loaders: Vec<Loader>,
    pub game_versions: Vec<String>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    pub client_only: bool,
    pub license: Option<String>,
    pub url: String,
    pub body: String,
    pub body_format: TextFormat,
    pub gallery: Vec<GalleryImage>,
    pub links: Vec<ProjectLink>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ModpackVersion {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub release_type: ReleaseType,
    /// Minecraft releases, newest first.
    pub game_versions: Vec<String>,
    pub loaders: Vec<Loader>,
    pub published: chrono::DateTime<chrono::Utc>,
    pub downloads: i64,
    pub file_name: String,
    pub file_size: i64,
    /// CurseForge blocks third-party downloads of this file, so it cannot be installed.
    pub downloadable: bool,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct VersionDetails {
    #[schema(inline)]
    pub version: ModpackVersion,
    pub changelog: String,
    pub changelog_format: TextFormat,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct Filters {
    pub categories: Vec<Category>,
    /// Minecraft releases, newest first.
    pub game_versions: Vec<String>,
    pub loaders: Vec<Loader>,
}

/// The archive of a modpack version, resolved on the backend so the install
/// script never downloads a client-supplied URL.
pub struct PackFile {
    pub version_name: String,
    pub url: String,
    pub sha1: Option<String>,
}

/// Compares Minecraft versions numerically (`1.20.10` > `1.20.9`, `26.1` > `1.21.11`).
pub fn compare_game_versions(a: &str, b: &str) -> Ordering {
    let parse = |version: &str| -> Vec<u64> {
        version
            .split(['.', '-'])
            .map(|part| {
                part.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    };
    parse(a).cmp(&parse(b))
}

/// Whether a version string is a full Minecraft release (no snapshots or pre-releases).
pub fn is_release_version(version: &str) -> bool {
    static RELEASE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\d+\.\d+(\.\d+)?$").expect("invalid release regex"));
    RELEASE.is_match(version)
}

/// Keeps release versions only, deduplicated and sorted newest first.
pub fn sort_game_versions<I: IntoIterator<Item = String>>(versions: I) -> Vec<String> {
    let mut versions: Vec<String> = versions
        .into_iter()
        .filter(|version| is_release_version(version))
        .collect();
    versions.sort_by(|a, b| compare_game_versions(b, a));
    versions.dedup();
    versions
}

pub fn sorted_loaders<I: IntoIterator<Item = Loader>>(loaders: I) -> Vec<Loader> {
    let mut loaders: Vec<Loader> = loaders.into_iter().collect();
    loaders.sort();
    loaders.dedup();
    loaders
}

/// Validates that `url` is an https URL on one of `hosts`.
pub fn is_https_url_on(url: &str, hosts: &[&str]) -> bool {
    url::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https" && url.host_str().is_some_and(|host| hosts.contains(&host))
    })
}

pub fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_versions_sort_numerically_and_drop_snapshots() {
        let sorted = sort_game_versions(
            ["1.20.9", "1.20.10", "26.1", "1.21.11", "25w14a", "1.21-pre1", "1.8", "1.20.10"]
                .map(String::from),
        );
        assert_eq!(sorted, ["26.1", "1.21.11", "1.20.10", "1.20.9", "1.8"]);
    }

    #[test]
    fn pack_urls_require_exact_https_hosts() {
        let hosts = ["cdn.modrinth.com"];
        assert!(is_https_url_on("https://cdn.modrinth.com/data/a/b.mrpack", &hosts));
        assert!(!is_https_url_on("http://cdn.modrinth.com/data/a/b.mrpack", &hosts));
        assert!(!is_https_url_on("https://cdn.modrinth.com.evil.test/b.mrpack", &hosts));
    }
}
