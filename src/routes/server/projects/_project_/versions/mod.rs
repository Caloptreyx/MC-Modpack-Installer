use super::State;
use crate::providers::{Loader, ModpackVersion, ReleaseType, sort_game_versions, sorted_loaders};
use utoipa_axum::{router::OpenApiRouter, routes};

mod _version_;

#[derive(Default)]
struct VersionFilter {
    search: Option<String>,
    loader: Option<Loader>,
    game_version: Option<String>,
    release_type: Option<ReleaseType>,
}

impl VersionFilter {
    fn matches(&self, version: &ModpackVersion, check_loader: bool, check_game_version: bool) -> bool {
        if let Some(search) = &self.search
            && !version.name.to_lowercase().contains(search)
            && !version.version_number.to_lowercase().contains(search)
        {
            return false;
        }
        if check_loader
            && let Some(loader) = self.loader
            && !version.loaders.contains(&loader)
        {
            return false;
        }
        if check_game_version
            && let Some(game_version) = &self.game_version
            && !version.game_versions.contains(game_version)
        {
            return false;
        }
        self.release_type
            .is_none_or(|release_type| version.release_type == release_type)
    }
}

/// Filters versions and computes the loader and game version options. Each
/// option list ignores its own filter so selecting a loader still lists every
/// loader available for the chosen Minecraft version, and vice versa.
fn filter_versions<'a>(
    versions: &'a [ModpackVersion],
    filter: &VersionFilter,
) -> (Vec<&'a ModpackVersion>, Vec<Loader>, Vec<String>) {
    let matching = versions
        .iter()
        .filter(|version| filter.matches(version, true, true))
        .collect();
    let loaders = sorted_loaders(
        versions
            .iter()
            .filter(|version| filter.matches(version, false, true))
            .flat_map(|version| version.loaders.iter().copied()),
    );
    let game_versions = sort_game_versions(
        versions
            .iter()
            .filter(|version| filter.matches(version, true, false))
            .flat_map(|version| version.game_versions.iter().cloned()),
    );

    (matching, loaders, game_versions)
}

mod get {
    use super::VersionFilter;
    use crate::providers::{Loader, ModpackVersion, ReleaseType};
    use axum::{
        extract::{Path, Query},
        http::StatusCode,
    };
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{Pagination, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::{IntoParams, ToSchema};

    fn default_per_page() -> i64 {
        10
    }

    #[derive(ToSchema, IntoParams, Validate, Deserialize)]
    #[into_params(parameter_in = Query)]
    pub struct Params {
        #[garde(range(min = 1))]
        #[serde(default = "Pagination::default_page")]
        page: i64,
        #[garde(range(min = 1, max = 100))]
        #[serde(default = "default_per_page")]
        per_page: i64,
        #[garde(length(chars, max = 128))]
        search: Option<String>,
        #[garde(skip)]
        loader: Option<Loader>,
        #[garde(length(chars, max = 32))]
        game_version: Option<String>,
        #[garde(skip)]
        release_type: Option<ReleaseType>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        versions: Pagination<ModpackVersion>,
        /// Loaders available with the other filters applied.
        loaders: Vec<Loader>,
        /// Minecraft versions available with the other filters applied, newest first.
        game_versions: Vec<String>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "provider" = String,
            description = "The modpack platform, `modrinth` or `curseforge`",
            example = "modrinth",
        ),
        (
            "project" = String,
            description = "The project id (or Modrinth slug)",
            example = "1KVo5zza",
        ),
        Params,
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path((_server, provider, project)): Path<(String, String, String)>,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&params) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("modpacks.read")?;

        let (client, project) = super::super::super::resolve(&state, &provider, &project).await?;
        let versions = client.versions(&state, &project).await?;

        let filter = VersionFilter {
            search: params
                .search
                .map(|search| search.trim().to_lowercase())
                .filter(|search| !search.is_empty()),
            loader: params.loader,
            game_version: params
                .game_version
                .map(|version| version.trim().to_string())
                .filter(|version| !version.is_empty()),
            release_type: params.release_type,
        };
        let (matching, loaders, game_versions) = super::filter_versions(&versions, &filter);

        let total = matching.len() as i64;
        let data = matching
            .into_iter()
            .skip(((params.page - 1) * params.per_page) as usize)
            .take(params.per_page as usize)
            .cloned()
            .collect();

        ApiResponse::new_serialized(Response {
            versions: Pagination {
                total,
                per_page: params.per_page,
                page: params.page,
                data,
            },
            loaders,
            game_versions,
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/{version}", _version_::router(state))
        .with_state(state.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(name: &str, loaders: &[Loader], game_versions: &[&str], release_type: ReleaseType) -> ModpackVersion {
        ModpackVersion {
            id: name.to_string(),
            name: name.to_string(),
            version_number: name.to_string(),
            release_type,
            game_versions: game_versions.iter().map(|v| v.to_string()).collect(),
            loaders: loaders.to_vec(),
            published: chrono::Utc::now(),
            downloads: 0,
            file_name: format!("{name}.mrpack"),
            file_size: 0,
            downloadable: true,
        }
    }

    #[test]
    fn option_lists_ignore_their_own_filter() {
        let versions = [
            version("a", &[Loader::Fabric], &["1.21.1"], ReleaseType::Release),
            version("b", &[Loader::Neoforge], &["1.21.1"], ReleaseType::Beta),
            version("c", &[Loader::Forge], &["1.20.1"], ReleaseType::Release),
        ];
        let filter = VersionFilter {
            loader: Some(Loader::Fabric),
            game_version: Some("1.21.1".into()),
            ..Default::default()
        };
        let (matching, loaders, game_versions) = filter_versions(&versions, &filter);

        assert_eq!(matching.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), ["a"]);
        // every loader offering 1.21.1, every game version offered with fabric
        assert_eq!(loaders, [Loader::Fabric, Loader::Neoforge]);
        assert_eq!(game_versions, ["1.21.1"]);
    }

    #[test]
    fn search_and_release_type_narrow_everything() {
        let versions = [
            version("Pack 2.0", &[Loader::Fabric], &["1.21.1"], ReleaseType::Release),
            version("Pack 2.1-beta", &[Loader::Fabric], &["1.21.4"], ReleaseType::Beta),
        ];
        let filter = VersionFilter {
            search: Some("2.1".into()),
            release_type: Some(ReleaseType::Beta),
            ..Default::default()
        };
        let (matching, _, game_versions) = filter_versions(&versions, &filter);

        assert_eq!(matching.len(), 1);
        assert_eq!(game_versions, ["1.21.4"]);
    }
}
