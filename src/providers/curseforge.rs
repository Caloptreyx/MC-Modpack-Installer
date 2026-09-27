//! CurseForge Core API client (https://docs.curseforge.com/rest-api/).
//!
//! Every request needs the API key configured by an administrator; it never
//! leaves the backend except for the install container, which needs it to
//! resolve the files of the pack.

use super::{
    CACHE_TTL, CLIENT, Category, Filters, GalleryImage, LONG_CACHE_TTL, Loader, ModpackDetails,
    ModpackSummary, ModpackVersion, PackFile, ProjectLink, Provider, ReleaseType, SearchQuery,
    SortMode, TextFormat, VersionDetails, fetch_json, is_https_url_on, non_empty,
    sort_game_versions, sorted_loaders, user_error,
};
use axum::http::StatusCode;
use serde::Deserialize;
use shared::{State, models::Pagination};

const API: &str = "https://api.curseforge.com";
const PLATFORM: &str = "CurseForge";
const MINECRAFT_GAME_ID: &str = "432";
const MODPACKS_CLASS_ID: i64 = 4471;
/// CurseForge rejects searches where `index + pageSize` exceeds this.
const MAX_SEARCH_WINDOW: i64 = 10_000;
/// Upper bound of files loaded for a single modpack.
const MAX_FILES: i64 = 2_000;
const FILES_PAGE_SIZE: i64 = 50;
pub const PACK_HOSTS: &[&str] = &["edge.forgecdn.net", "mediafilez.forgecdn.net"];

type Timestamp = chrono::DateTime<chrono::Utc>;

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct PagedEnvelope<T> {
    data: Vec<T>,
    pagination: CfPagination,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfPagination {
    #[serde(default)]
    total_count: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mod {
    id: i64,
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    links: Links,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    thumbs_up_count: i64,
    class_id: Option<i64>,
    #[serde(default)]
    categories: Vec<CfCategory>,
    #[serde(default)]
    authors: Vec<Author>,
    logo: Option<Asset>,
    #[serde(default)]
    screenshots: Vec<Asset>,
    #[serde(default)]
    latest_files_indexes: Vec<FileIndex>,
    date_created: Option<Timestamp>,
    date_modified: Option<Timestamp>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Links {
    website_url: Option<String>,
    wiki_url: Option<String>,
    issues_url: Option<String>,
    source_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfCategory {
    id: i64,
    name: String,
    #[serde(default)]
    is_class: Option<bool>,
}

#[derive(Deserialize)]
struct Author {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Asset {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    thumbnail_url: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileIndex {
    game_version: String,
    mod_loader: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    id: i64,
    mod_id: i64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    release_type: i64,
    #[serde(default)]
    hashes: Vec<FileHash>,
    file_date: Timestamp,
    #[serde(default)]
    file_length: i64,
    #[serde(default)]
    download_count: f64,
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    is_server_pack: Option<bool>,
}

#[derive(Deserialize)]
struct FileHash {
    value: String,
    algo: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinecraftVersion {
    version_string: String,
}

fn request(key: &str, path: &str) -> reqwest::RequestBuilder {
    CLIENT
        .get(format!("{API}{path}"))
        .header("x-api-key", key)
        .header("Accept", "application/json")
}

/// CurseForge mod ids are numeric; reject anything else before building URLs.
fn parse_id(id: &str) -> Result<i64, anyhow::Error> {
    id.parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| user_error("invalid CurseForge id", StatusCode::NOT_FOUND))
}

fn loader_from_type(mod_loader: i64) -> Option<Loader> {
    match mod_loader {
        1 => Some(Loader::Forge),
        4 => Some(Loader::Fabric),
        5 => Some(Loader::Quilt),
        6 => Some(Loader::Neoforge),
        _ => None,
    }
}

fn loader_type(loader: Loader) -> &'static str {
    match loader {
        Loader::Forge => "1",
        Loader::Fabric => "4",
        Loader::Quilt => "5",
        Loader::Neoforge => "6",
    }
}

fn icon_of(logo: Option<Asset>) -> Option<String> {
    logo.and_then(|logo| non_empty(logo.thumbnail_url).or(non_empty(logo.url)))
}

fn project_url(project: &Mod) -> String {
    non_empty(project.links.website_url.clone())
        .unwrap_or_else(|| format!("https://www.curseforge.com/minecraft/modpacks/{}", project.slug))
}

fn summarize(project: Mod) -> ModpackSummary {
    let url = project_url(&project);
    ModpackSummary {
        provider: Provider::Curseforge,
        id: project.id.to_string(),
        slug: project.slug,
        name: project.name,
        summary: project.summary,
        author: project
            .authors
            .into_iter()
            .next()
            .map(|author| author.name)
            .unwrap_or_default(),
        icon_url: icon_of(project.logo),
        downloads: project.download_count as i64,
        follows: project.thumbs_up_count,
        categories: project
            .categories
            .into_iter()
            .filter(|category| category.is_class != Some(true))
            .map(|category| category.name)
            .collect(),
        loaders: sorted_loaders(
            project
                .latest_files_indexes
                .iter()
                .filter_map(|index| index.mod_loader.and_then(loader_from_type)),
        ),
        game_versions: sort_game_versions(
            project
                .latest_files_indexes
                .into_iter()
                .map(|index| index.game_version),
        ),
        updated: project.date_modified,
        client_only: false,
        url,
    }
}

pub async fn search(
    state: &State,
    key: &str,
    query: &SearchQuery,
) -> Result<Pagination<ModpackSummary>, anyhow::Error> {
    let index = (query.page - 1) * query.per_page;
    if index + query.per_page > MAX_SEARCH_WINDOW {
        return Ok(Pagination {
            total: MAX_SEARCH_WINDOW,
            per_page: query.per_page,
            page: query.page,
            data: Vec::new(),
        });
    }

    let mut params: Vec<(&str, String)> = vec![
        ("gameId", MINECRAFT_GAME_ID.to_string()),
        ("classId", MODPACKS_CLASS_ID.to_string()),
        ("sortOrder", "desc".to_string()),
        ("index", index.to_string()),
        ("pageSize", query.per_page.to_string()),
        (
            "sortField",
            match query.sort {
                // CurseForge has no relevance order; popularity is what its website uses.
                SortMode::Relevance => "2",
                SortMode::Downloads => "6",
                SortMode::Follows => "12",
                SortMode::Newest => "11",
                SortMode::Updated => "3",
            }
            .to_string(),
        ),
    ];
    if let Some(text) = &query.query {
        params.push(("searchFilter", text.clone()));
    }
    if let Some(game_version) = &query.game_version {
        params.push(("gameVersion", game_version.clone()));
    }
    if let Some(loader) = query.loader {
        params.push(("modLoaderType", loader_type(loader).to_string()));
    }
    if let Some(category) = &query.category {
        params.push(("categoryId", parse_id(category)?.to_string()));
    }

    let cache_key = format!(
        "modpacks::curseforge::search::{}",
        params
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("&")
    );
    let key = key.to_string();

    let (total, data): (i64, Vec<ModpackSummary>) = state
        .cache
        .cached(&cache_key, CACHE_TTL, || async move {
            let response: PagedEnvelope<Mod> =
                fetch_json(request(&key, "/v1/mods/search").query(&params), PLATFORM).await?;

            Ok::<_, anyhow::Error>((
                response.pagination.total_count.min(MAX_SEARCH_WINDOW),
                response.data.into_iter().map(summarize).collect(),
            ))
        })
        .await?;

    Ok(Pagination {
        total,
        per_page: query.per_page,
        page: query.page,
        data,
    })
}

pub async fn filters(state: &State, key: &str) -> Result<Filters, anyhow::Error> {
    let key = key.to_string();
    state
        .cache
        .cached("modpacks::curseforge::filters", LONG_CACHE_TTL, || async move {
            let (categories, versions): (Envelope<Vec<CfCategory>>, Envelope<Vec<MinecraftVersion>>) =
                tokio::try_join!(
                    fetch_json(
                        request(&key, "/v1/categories").query(&[
                            ("gameId", MINECRAFT_GAME_ID),
                            ("classId", &MODPACKS_CLASS_ID.to_string()),
                        ]),
                        PLATFORM,
                    ),
                    fetch_json(request(&key, "/v1/minecraft/version"), PLATFORM),
                )?;

            let mut categories: Vec<Category> = categories
                .data
                .into_iter()
                .filter(|category| category.is_class != Some(true))
                .map(|category| Category {
                    id: category.id.to_string(),
                    name: category.name,
                })
                .collect();
            categories.sort_by(|a, b| a.name.cmp(&b.name));

            Ok::<_, anyhow::Error>(Filters {
                categories,
                game_versions: sort_game_versions(
                    versions.data.into_iter().map(|version| version.version_string),
                ),
                loaders: Loader::ALL.to_vec(),
            })
        })
        .await
}

async fn fetch_mod(key: &str, id: i64) -> Result<Mod, anyhow::Error> {
    let project: Envelope<Mod> = fetch_json(request(key, &format!("/v1/mods/{id}")), PLATFORM).await?;
    if project.data.class_id != Some(MODPACKS_CLASS_ID) {
        return Err(user_error("this project is not a modpack", StatusCode::NOT_FOUND));
    }
    Ok(project.data)
}

pub async fn project(state: &State, key: &str, id: &str) -> Result<ModpackDetails, anyhow::Error> {
    let id = parse_id(id)?;
    let key = key.to_string();
    state
        .cache
        .cached(&format!("modpacks::curseforge::project::{id}"), CACHE_TTL, || async move {
            let (project, description): (Mod, Envelope<String>) = tokio::try_join!(
                fetch_mod(&key, id),
                fetch_json(request(&key, &format!("/v1/mods/{id}/description")), PLATFORM),
            )?;

            let mut links = Vec::new();
            for (kind, url) in [
                ("website", project.links.website_url.clone()),
                ("source", project.links.source_url.clone()),
                ("issues", project.links.issues_url.clone()),
                ("wiki", project.links.wiki_url.clone()),
            ] {
                if let Some(url) = non_empty(url) {
                    links.push(ProjectLink {
                        kind: kind.to_string(),
                        url,
                    });
                }
            }

            let created = project.date_created;
            let authors: Vec<String> = project.authors.iter().map(|author| author.name.clone()).collect();
            let gallery = project
                .screenshots
                .iter()
                .filter_map(|image| {
                    Some(GalleryImage {
                        url: non_empty(image.url.clone())?,
                        title: non_empty(image.title.clone()),
                        description: non_empty(image.description.clone()),
                    })
                })
                .collect();
            let summary = summarize(project);

            Ok::<_, anyhow::Error>(ModpackDetails {
                provider: Provider::Curseforge,
                id: summary.id,
                slug: summary.slug,
                name: summary.name,
                summary: summary.summary,
                authors,
                icon_url: summary.icon_url,
                downloads: summary.downloads,
                follows: summary.follows,
                categories: summary.categories,
                loaders: summary.loaders,
                game_versions: summary.game_versions,
                created,
                updated: summary.updated,
                client_only: false,
                license: None,
                url: summary.url,
                body: description.data,
                body_format: TextFormat::Html,
                gallery,
                links,
            })
        })
        .await
}

fn map_file(file: File) -> ModpackVersion {
    let (mut loaders, mut game_versions) = (Vec::new(), Vec::new());
    for value in file.game_versions {
        match Loader::parse(&value) {
            Some(loader) => loaders.push(loader),
            None => game_versions.push(value),
        }
    }
    let name = file
        .file_name
        .strip_suffix(".zip")
        .unwrap_or(&file.file_name)
        .to_string();

    ModpackVersion {
        id: file.id.to_string(),
        name: if file.display_name.is_empty() {
            name.clone()
        } else {
            file.display_name
        },
        version_number: name,
        release_type: match file.release_type {
            2 => ReleaseType::Beta,
            3 => ReleaseType::Alpha,
            _ => ReleaseType::Release,
        },
        game_versions: sort_game_versions(game_versions),
        loaders: sorted_loaders(loaders),
        published: file.file_date,
        downloads: file.download_count as i64,
        downloadable: file
            .download_url
            .as_deref()
            .is_some_and(|url| is_https_url_on(url, PACK_HOSTS)),
        file_name: file.file_name,
        file_size: file.file_length,
    }
}

pub async fn versions(state: &State, key: &str, project_id: &str) -> Result<Vec<ModpackVersion>, anyhow::Error> {
    let id = parse_id(project_id)?;
    let key = key.to_string();
    state
        .cache
        .cached(&format!("modpacks::curseforge::versions::{id}"), CACHE_TTL, || async move {
            let mut files = Vec::new();
            let mut index = 0;
            loop {
                let page: PagedEnvelope<File> = fetch_json(
                    request(&key, &format!("/v1/mods/{id}/files")).query(&[
                        ("index", index.to_string()),
                        ("pageSize", FILES_PAGE_SIZE.to_string()),
                    ]),
                    PLATFORM,
                )
                .await?;
                let received = page.data.len() as i64;
                files.extend(page.data);
                index += FILES_PAGE_SIZE;
                if received < FILES_PAGE_SIZE || index >= page.pagination.total_count || index >= MAX_FILES {
                    break;
                }
            }

            let mut versions: Vec<ModpackVersion> = files
                .into_iter()
                .filter(|file| file.is_server_pack != Some(true))
                .map(map_file)
                .collect();
            versions.sort_by_key(|version| std::cmp::Reverse(version.published));

            Ok::<_, anyhow::Error>(versions)
        })
        .await
}

async fn fetch_file(key: &str, project_id: i64, file_id: i64) -> Result<File, anyhow::Error> {
    let file: Envelope<File> = fetch_json(
        request(key, &format!("/v1/mods/{project_id}/files/{file_id}")),
        PLATFORM,
    )
    .await?;
    if file.data.mod_id != project_id || file.data.is_server_pack == Some(true) {
        return Err(user_error(
            "this version does not belong to the modpack",
            StatusCode::NOT_FOUND,
        ));
    }
    Ok(file.data)
}

pub async fn version(
    state: &State,
    key: &str,
    project_id: &str,
    version_id: &str,
) -> Result<VersionDetails, anyhow::Error> {
    let (project_id, file_id) = (parse_id(project_id)?, parse_id(version_id)?);
    let key = key.to_string();
    state
        .cache
        .cached(
            &format!("modpacks::curseforge::version::{project_id}::{file_id}"),
            CACHE_TTL,
            || async move {
                let (file, changelog): (File, Envelope<String>) = tokio::try_join!(
                    fetch_file(&key, project_id, file_id),
                    fetch_json(
                        request(&key, &format!("/v1/mods/{project_id}/files/{file_id}/changelog")),
                        PLATFORM,
                    ),
                )?;

                Ok::<_, anyhow::Error>(VersionDetails {
                    version: map_file(file),
                    changelog: changelog.data,
                    changelog_format: TextFormat::Html,
                })
            },
        )
        .await
}

pub async fn pack_file(key: &str, project_id: &str, version_id: &str) -> Result<PackFile, anyhow::Error> {
    let file = fetch_file(key, parse_id(project_id)?, parse_id(version_id)?).await?;
    let url = file
        .download_url
        .filter(|url| is_https_url_on(url, PACK_HOSTS))
        .ok_or_else(|| {
            user_error(
                "the author of this modpack does not allow third-party downloads of this version",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
        })?;

    Ok(PackFile {
        version_name: if file.display_name.is_empty() {
            file.file_name
        } else {
            file.display_name
        },
        url,
        sha1: file
            .hashes
            .into_iter()
            .find(|hash| hash.algo == 1)
            .map(|hash| hash.value),
    })
}

/// Checks an API key against CurseForge without touching the stored settings.
pub async fn verify_key(key: &str) -> Result<(), anyhow::Error> {
    let response = CLIENT
        .get(format!("{API}/v1/games/{MINECRAFT_GAME_ID}"))
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|err| user_error(format!("CurseForge is unreachable: {err}"), StatusCode::BAD_GATEWAY))?;

    match response.status() {
        status if status.is_success() => Ok(()),
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => Err(user_error(
            "CurseForge rejected the API key",
            StatusCode::BAD_REQUEST,
        )),
        status => Err(user_error(
            format!("CurseForge returned HTTP {status}"),
            StatusCode::BAD_GATEWAY,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_game_versions_are_split_into_loaders_and_releases() {
        let version = map_file(File {
            id: 7,
            mod_id: 1,
            display_name: String::new(),
            file_name: "Pack-1.2.3.zip".into(),
            release_type: 2,
            hashes: Vec::new(),
            file_date: chrono::Utc::now(),
            file_length: 10,
            download_count: 3.0,
            download_url: Some("https://edge.forgecdn.net/files/1/2/Pack-1.2.3.zip".into()),
            game_versions: ["1.20.1", "NeoForge", "Forge", "Client", "Server"].map(String::from).to_vec(),
            is_server_pack: None,
        });
        assert_eq!(version.loaders, [Loader::Forge, Loader::Neoforge]);
        assert_eq!(version.game_versions, ["1.20.1"]);
        assert_eq!(version.name, "Pack-1.2.3");
        assert!(matches!(version.release_type, ReleaseType::Beta));
        assert!(version.downloadable);
    }

    #[test]
    fn ids_must_be_positive_numbers() {
        assert!(parse_id("1234").is_ok());
        assert!(parse_id("0").is_err());
        assert!(parse_id("../1").is_err());
    }
}
