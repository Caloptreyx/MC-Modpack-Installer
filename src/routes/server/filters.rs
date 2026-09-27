use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::providers::{Client, Filters, Provider};
    use axum::extract::Query;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::{IntoParams, ToSchema};

    #[derive(ToSchema, IntoParams, Deserialize)]
    #[into_params(parameter_in = Query)]
    pub struct Params {
        provider: Provider,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        filters: Filters,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
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
        permissions.has_server_permission("modpacks.read")?;

        let settings = crate::settings::load(&state).await?;
        let filters = Client::new(&settings, params.provider)?.filters(&state).await?;

        ApiResponse::new_serialized(Response { filters }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
