use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::providers::VersionDetails;
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
        version: VersionDetails,
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
        (
            "version" = String,
            description = "The version (Modrinth) or file (CurseForge) id",
            example = "nW5BtnR5",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path((_server, provider, project, version)): Path<(String, String, String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("modpacks.read")?;

        let (client, project) = super::super::super::super::resolve(&state, &provider, &project).await?;
        let version = client.version(&state, &project, &version).await?;

        ApiResponse::new_serialized(Response { version }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
