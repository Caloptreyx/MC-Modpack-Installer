//! Modrinth API v2 client (https://docs.modrinth.com/api/).

use super::{
    CACHE_TTL, CLIENT, Category, Filters, GalleryImage, LONG_CACHE_TTL, Loader, ModpackDetails,
    ModpackSummary, ModpackVersion, PackFile, ProjectLink, Provider, ReleaseType, SearchQuery,
    SortMode, TextFormat, VersionDetails, fetch_json, is_https_url_on, non_empty, sort_game_versions,
    sorted_loaders, user_error,
};
use axum::http::StatusCode;
use serde::Deserialize;
use shared::{State, models::Pagination};

const API: &str = "https://api.modrinth.com";
const PLATFORM: &str = "Modrinth";
pub const PACK_HOSTS: &[&str] = &["cdn.modrinth.com"];

type Timestamp = chrono::DateTime<chrono::Utc>;

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
    total_hits: i64,
}

#[derive(Deserialize)]
struct SearchHit {
    project_id: String,
    slug: Option<String>,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    display_categories: Vec<String>,
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    downloads: i64,
    #[serde(default)]
    follows: i64,
    icon_url: Option<String>,
    date_modified: Option<Timestamp>,
    server_side: Option<String>,
    #[serde(default)]
    environment: Vec<String>,
}

#[derive(Deserialize)]
struct Project {
    id: String,
    slug: Option<String>,
    project_type: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    additional_categories: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    downloads: i64,
    #[serde(default)]
    followers: i64,
    icon_url: Option<String>,
    published: Option<Timestamp>,
    updated: Option<Timestamp>,
    server_side: Option<String>,
    #[serde(default)]
    environment: Vec<String>,
    license: Option<License>,
    source_url: Option<String>,
    issues_url: Option<String>,
    wiki_url: Option<String>,
    discord_url: Option<String>,
    #[serde(default)]
    donation_urls: Vec<DonationUrl>,
    #[serde(default)]
    gallery: Vec<Gallery>,
    organization: Option<String>,
}

#[derive(Deserialize)]
struct License {
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct DonationUrl {
    url: String,
}

#[derive(Deserialize)]
struct Gallery {
    url: String,
    title: Option<String>,
    description: Option<String>,
    #[serde(default)]
    featured: bool,
    #[serde(default)]
    ordering: i64,
}

#[derive(Deserialize)]
struct TeamMember {
    user: TeamUser,
    #[serde(default)]
    ordering: i64,
}

#[derive(Deserialize)]
struct TeamUser {
    username: String,
}

#[derive(Deserialize)]
struct Organization {
    name: String,
}

#[derive(Deserialize)]
struct Version {
    id: String,
    project_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_number: String,
    changelog: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    version_type: String,
    date_published: Timestamp,
    #[serde(default)]
    downloads: i64,
    #[serde(default)]
    files: Vec<VersionFile>,
}

#[derive(Deserialize)]
struct VersionFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: i64,
    #[serde(default)]
    hashes: FileHashes,
}

#[derive(Deserialize, Default)]
struct FileHashes {
    sha1: Option<String>,
}

#[derive(Deserialize)]
struct CategoryTag {
    name: String,
    project_type: String,
    #[serde(default)]
    header: String,
}

#[derive(Deserialize)]
struct GameVersionTag {
    version: String,
    version_type: String,
}

fn project_url(slug: &str) -> String {
    format!("https://modrinth.com/modpack/{slug}")
}

/// Whether a pack cannot run on a dedicated server. Modrinth reports one
/// `environment` entry per distinct version environment; `server_side` is the
/// legacy field and only used when `environment` is missing.
fn is_client_only(environment: &[String], server_side: Option<&str>) -> bool {
    if environment.is_empty() {
        return server_side == Some("unsupported");
    }
    environment
        .iter()
        .all(|value| matches!(value.as_str(), "client_only" | "singleplayer_only"))
}

fn loaders_of(values: &[String]) -> Vec<Loader> {
    sorted_loaders(values.iter().filter_map(|value| Loader::parse(value)))
}

fn category_label(name: &str) -> String {
    let mut label = String::with_capacity(name.len());
    for (index, word) in name.split('-').enumerate() {
        if index > 0 {
            label.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            label.extend(first.to_uppercase());
            label.push_str(chars.as_str());
        }
    }
    label
}

pub async fn search(
    state: &State,
    query: &SearchQuery,
) -> Result<Pagination<ModpackSummary>, anyhow::Error> {
    let mut facets = vec![vec!["project_type:modpack".to_string()]];
    if let Some(loader) = query.loader {
        facets.push(vec![format!("categories:{}", loader.as_str())]);
    }
    if let Some(game_version) = &query.game_version {
        facets.push(vec![format!("versions:{game_version}")]);
    }
    if let Some(category) = &query.category {
        facets.push(vec![format!("categories:{category}")]);
    }
    if query.hide_client_only {
        facets.push(vec![
            "server_side:required".to_string(),
            "server_side:optional".to_string(),
        ]);
    }
    let facets = serde_json::to_string(&facets)?;
    let index = match query.sort {
        SortMode::Relevance => "relevance",
        SortMode::Downloads => "downloads",
        SortMode::Follows => "follows",
        SortMode::Newest => "newest",
        SortMode::Updated => "updated",
    };
    let offset = (query.page - 1) * query.per_page;
    let text = query.query.clone().unwrap_or_default();

    let key = format!(
        "modpacks::modrinth::search::{index}::{offset}::{}::{facets}::{text}",
        query.per_page
    );
    let per_page = query.per_page;

    let (total, hits): (i64, Vec<ModpackSummary>) = state
        .cache
        .cached(&key, CACHE_TTL, || async move {
            let response: SearchResponse = fetch_json(
                CLIENT.get(format!("{API}/v2/search")).query(&[
                    ("query", text.as_str()),
                    ("facets", facets.as_str()),
                    ("index", index),
                    ("offset", &offset.to_string()),
                    ("limit", &per_page.to_string()),
                ]),
                PLATFORM,
            )
            .await?;

            let hits = response
                .hits
                .into_iter()
                .map(|hit| {
                    let slug = hit.slug.unwrap_or_else(|| hit.project_id.clone());
                    ModpackSummary {
                        provider: Provider::Modrinth,
                        url: project_url(&slug),
                        loaders: loaders_of(&hit.categories),
                        categories: hit
                            .display_categories
                            .iter()
                            .filter(|category| Loader::parse(category).is_none())
                            .map(|category| category_label(category))
                            .collect(),
                        id: hit.project_id,
                        slug,
                        name: hit.title,
                        summary: hit.description,
                        author: hit.author,
                        icon_url: non_empty(hit.icon_url),
                        downloads: hit.downloads,
                        follows: hit.follows,
                        game_versions: sort_game_versions(hit.versions),
                        updated: hit.date_modified,
                        client_only: is_client_only(&hit.environment, hit.server_side.as_deref()),
                    }
                })
                .collect();

            Ok::<_, anyhow::Error>((response.total_hits, hits))
        })
        .await?;

    Ok(Pagination {
        total,
        per_page: query.per_page,
        page: query.page,
        data: hits,
    })
}

pub async fn filters(state: &State) -> Result<Filters, anyhow::Error> {
    state
        .cache
        .cached("modpacks::modrinth::filters", LONG_CACHE_TTL, || async {
            let (categories, game_versions): (Vec<CategoryTag>, Vec<GameVersionTag>) = tokio::try_join!(
                fetch_json(CLIENT.get(format!("{API}/v2/tag/category")), PLATFORM),
                fetch_json(CLIENT.get(format!("{API}/v2/tag/game_version")), PLATFORM),
            )?;

            let mut categories: Vec<Category> = categories
                .into_iter()
                .filter(|category| {
                    category.project_type == "modpack"
                        && category.header == "categories"
                        && Loader::parse(&category.name).is_none()
                })
                .map(|category| Category {
                    name: category_label(&category.name),
                    id: category.name,
                })
                .collect();
            categories.sort_by(|a, b| a.name.cmp(&b.name));

            Ok::<_, anyhow::Error>(Filters {
                categories,
                game_versions: sort_game_versions(
                    game_versions
                        .into_iter()
                        .filter(|version| version.version_type == "release")
                        .map(|version| version.version),
                ),
                loaders: Loader::ALL.to_vec(),
            })
        })
        .await
}

pub async fn project(state: &State, id: &str) -> Result<ModpackDetails, anyhow::Error> {
    let id = id.to_string();
    state
        .cache
        .cached(&format!("modpacks::modrinth::project::{id}"), CACHE_TTL, || async move {
            let project: Project = fetch_json(
                CLIENT.get(format!("{API}/v2/project/{}", urlencoding(&id))),
                PLATFORM,
            )
            .await?;
            if project.project_type != "modpack" {
                return Err(user_error("this project is not a modpack", StatusCode::NOT_FOUND));
            }

            let authors = match &project.organization {
                Some(organization) => fetch_json::<Organization>(
                    CLIENT.get(format!("{API}/v3/organization/{}", urlencoding(organization))),
                    PLATFORM,
                )
                .await
                .map(|organization| vec![organization.name])
                .unwrap_or_default(),
                None => {
                    let mut members: Vec<TeamMember> = fetch_json(
                        CLIENT.get(format!("{API}/v2/project/{}/members", project.id)),
                        PLATFORM,
                    )
                    .await
                    .unwrap_or_default();
                    members.sort_by_key(|member| member.ordering);
                    members.into_iter().map(|member| member.user.username).collect()
                }
            };

            let mut gallery = project.gallery;
            gallery.sort_by_key(|image| (!image.featured, image.ordering));

            let mut links = Vec::new();
            for (kind, url) in [
                ("source", project.source_url),
                ("issues", project.issues_url),
                ("wiki", project.wiki_url),
                ("discord", project.discord_url),
            ] {
                if let Some(url) = non_empty(url) {
                    links.push(ProjectLink {
                        kind: kind.to_string(),
                        url,
                    });
                }
            }
            links.extend(project.donation_urls.into_iter().map(|donation| ProjectLink {
                kind: "donation".to_string(),
                url: donation.url,
            }));

            let slug = project.slug.unwrap_or_else(|| project.id.clone());
            let mut loader_names = project.loaders;
            loader_names.extend(project.categories.iter().cloned());

            Ok(ModpackDetails {
                provider: Provider::Modrinth,
                url: project_url(&slug),
                loaders: loaders_of(&loader_names),
                categories: project
                    .categories
                    .iter()
                    .chain(project.additional_categories.iter())
                    .filter(|category| Loader::parse(category).is_none())
                    .map(|category| category_label(category))
                    .collect(),
                id: project.id,
                slug,
                name: project.title,
                summary: project.description,
                authors,
                icon_url: non_empty(project.icon_url),
                downloads: project.downloads,
                follows: project.followers,
                game_versions: sort_game_versions(project.game_versions),
                created: project.published,
                updated: project.updated,
                client_only: is_client_only(&project.environment, project.server_side.as_deref()),
                license: project.license.map(|license| {
                    if license.name.is_empty() {
                        license.id
                    } else {
                        license.name
                    }
                }),
                body: project.body,
                body_format: TextFormat::Markdown,
                gallery: gallery
                    .into_iter()
                    .map(|image| GalleryImage {
                        url: image.url,
                        title: non_empty(image.title),
                        description: non_empty(image.description),
                    })
                    .collect(),
                links,
            })
        })
        .await
}

fn primary_file(files: &[VersionFile]) -> Option<&VersionFile> {
    files
        .iter()
        .find(|file| file.primary && file.filename.ends_with(".mrpack"))
        .or_else(|| files.iter().find(|file| file.filename.ends_with(".mrpack")))
}

fn map_version(version: Version) -> ModpackVersion {
    let (file_name, file_size) = primary_file(&version.files)
        .map(|file| (file.filename.clone(), file.size))
        .unwrap_or_default();

    ModpackVersion {
        name: if version.name.is_empty() {
            version.version_number.clone()
        } else {
            version.name
        },
        id: version.id,
        version_number: version.version_number,
        release_type: match version.version_type.as_str() {
            "beta" => ReleaseType::Beta,
            "alpha" => ReleaseType::Alpha,
            _ => ReleaseType::Release,
        },
        game_versions: sort_game_versions(version.game_versions),
        loaders: loaders_of(&version.loaders),
        published: version.date_published,
        downloads: version.downloads,
        downloadable: !file_name.is_empty(),
        file_name,
        file_size,
    }
}

pub async fn versions(state: &State, project_id: &str) -> Result<Vec<ModpackVersion>, anyhow::Error> {
    let project_id = project_id.to_string();
    state
        .cache
        .cached(
            &format!("modpacks::modrinth::versions::{project_id}"),
            CACHE_TTL,
            || async move {
                let versions: Vec<Version> = fetch_json(
                    CLIENT.get(format!("{API}/v2/project/{}/version", urlencoding(&project_id))),
                    PLATFORM,
                )
                .await?;

                let mut versions: Vec<ModpackVersion> = versions.into_iter().map(map_version).collect();
                versions.sort_by_key(|version| std::cmp::Reverse(version.published));

                Ok::<_, anyhow::Error>(versions)
            },
        )
        .await
}

async fn fetch_version(project_id: &str, version_id: &str) -> Result<Version, anyhow::Error> {
    let version: Version = fetch_json(
        CLIENT.get(format!("{API}/v2/version/{}", urlencoding(version_id))),
        PLATFORM,
    )
    .await?;
    if version.project_id != project_id {
        return Err(user_error(
            "this version does not belong to the modpack",
            StatusCode::NOT_FOUND,
        ));
    }
    Ok(version)
}

pub async fn version(
    state: &State,
    project_id: &str,
    version_id: &str,
) -> Result<VersionDetails, anyhow::Error> {
    let (project_id, version_id) = (project_id.to_string(), version_id.to_string());
    state
        .cache
        .cached(
            &format!("modpacks::modrinth::version::{project_id}::{version_id}"),
            CACHE_TTL,
            || async move {
                let mut version = fetch_version(&project_id, &version_id).await?;
                let changelog = version.changelog.take().unwrap_or_default();

                Ok::<_, anyhow::Error>(VersionDetails {
                    version: map_version(version),
                    changelog,
                    changelog_format: TextFormat::Markdown,
                })
            },
        )
        .await
}

pub async fn pack_file(project_id: &str, version_id: &str) -> Result<PackFile, anyhow::Error> {
    let version = fetch_version(project_id, version_id).await?;
    let file = primary_file(&version.files).ok_or_else(|| {
        user_error(
            "this version has no .mrpack file",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
    })?;
    if !is_https_url_on(&file.url, PACK_HOSTS) {
        return Err(user_error(
            "the modpack file is not hosted on the Modrinth CDN",
            StatusCode::UNPROCESSABLE_ENTITY,
        ));
    }

    Ok(PackFile {
        version_name: if version.version_number.is_empty() {
            version.name.clone()
        } else {
            version.version_number.clone()
        },
        url: file.url.clone(),
        sha1: file.hashes.sha1.clone(),
    })
}

/// Percent-encodes a single path segment.
fn urlencoding(segment: &str) -> String {
    url::form_urlencoded::byte_serialize(segment.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_only_needs_every_environment_to_be_client_side() {
        let env = |values: &[&str]| values.iter().map(|value| value.to_string()).collect::<Vec<_>>();
        assert!(is_client_only(&env(&["client_only"]), Some("unsupported")));
        assert!(!is_client_only(&env(&["client_only", "dedicated_server_only"]), Some("unsupported")));
        assert!(!is_client_only(&env(&["client_and_server"]), Some("required")));
        assert!(is_client_only(&[], Some("unsupported")));
        assert!(!is_client_only(&[], None));
    }

    #[test]
    fn category_labels_are_title_cased() {
        assert_eq!(category_label("tech"), "Tech");
        assert_eq!(category_label("kitchen-sink"), "Kitchen Sink");
    }
}
