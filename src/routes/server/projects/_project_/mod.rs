use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod versions;

mod get {
    use crate::providers::ModpackDetails;
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        modpack: ModpackDetails,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
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
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path((_server, provider, project)): Path<(String, String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("modpacks.read")?;

        let (_, modpack) = super::super::resolve(&state, &provider, &project).await?;

        ApiResponse::new_serialized(Response { modpack }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/versions", versions::router(state))
        .with_state(state.clone())
}
