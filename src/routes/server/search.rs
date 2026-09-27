use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::providers::{Client, Loader, ModpackSummary, Provider, SearchQuery, SortMode};
    use axum::{extract::Query, http::StatusCode};
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{Pagination, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::{IntoParams, ToSchema};

    fn default_per_page() -> i64 {
        20
    }

    #[derive(ToSchema, IntoParams, Validate, Deserialize)]
    #[into_params(parameter_in = Query)]
    pub struct Params {
        #[garde(skip)]
        provider: Provider,
        #[garde(range(min = 1))]
        #[serde(default = "Pagination::default_page")]
        page: i64,
        #[garde(range(min = 1, max = 50))]
        #[serde(default = "default_per_page")]
        per_page: i64,
        #[garde(length(chars, max = 128))]
        search: Option<String>,
        #[garde(skip)]
        loader: Option<Loader>,
        #[garde(length(chars, max = 32))]
        game_version: Option<String>,
        #[garde(length(chars, max = 64))]
        category: Option<String>,
        #[garde(skip)]
        #[serde(default)]
        sort: SortMode,
        #[garde(skip)]
        #[serde(default)]
        hide_client_only: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        modpacks: Pagination<ModpackSummary>,
    }

    fn filled(value: Option<String>) -> Option<String> {
        value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = SERVICE_UNAVAILABLE, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        Params,
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&params) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("modpacks.read")?;

        let settings = crate::settings::load(&state).await?;
        let client = Client::new(&settings, params.provider)?;
        let modpacks = client
            .search(
                &state,
                &SearchQuery {
                    query: filled(params.search),
                    loader: params.loader,
                    game_version: filled(params.game_version),
                    category: filled(params.category),
                    sort: params.sort,
                    page: params.page,
                    per_page: params.per_page,
                    hide_client_only: params.hide_client_only,
                },
            )
            .await?;

        ApiResponse::new_serialized(Response { modpacks }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
