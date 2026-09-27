use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod verify {
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        /// The key to check; omit it to check the stored key.
        #[garde(length(chars, max = 255))]
        #[schema(max_length = 255)]
        api_key: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("modpacks.manage")?;

        let key = match data.api_key.map(|key| key.trim().to_string()).filter(|key| !key.is_empty()) {
            Some(key) => key,
            None => match crate::settings::load(&state).await?.curseforge_api_key {
                Some(key) => key,
                None => {
                    return ApiResponse::error("no CurseForge API key is configured")
                        .with_status(StatusCode::BAD_REQUEST)
                        .ok();
                }
            },
        };

        crate::providers::curseforge::verify_key(&key).await?;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest(
            "/verify",
            OpenApiRouter::new()
                .routes(routes!(verify::route))
                .with_state(state.clone()),
        )
        .with_state(state.clone())
}
