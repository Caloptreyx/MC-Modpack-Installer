use super::State;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Settings as shown to administrators; the CurseForge API key is write-only.
#[derive(ToSchema, Serialize)]
pub struct AdminSettings {
    modrinth_enabled: bool,
    curseforge_enabled: bool,
    curseforge_configured: bool,
    allow_clean_install: bool,
    installer_image: String,
    excluded_mods: Vec<String>,
}

impl From<&crate::settings::ExtensionSettingsData> for AdminSettings {
    fn from(settings: &crate::settings::ExtensionSettingsData) -> Self {
        Self {
            modrinth_enabled: settings.modrinth_enabled,
            curseforge_enabled: settings.curseforge_enabled,
            curseforge_configured: settings
                .curseforge_api_key
                .as_deref()
                .is_some_and(|key| !key.is_empty()),
            allow_clean_install: settings.allow_clean_install,
            installer_image: settings.installer_image.clone(),
            excluded_mods: settings.excluded_mods.clone(),
        }
    }
}

mod get {
    use super::AdminSettings;
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
        settings: AdminSettings,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("modpacks.read")?;

        let settings = crate::settings::load(&state).await?;

        ApiResponse::new_serialized(Response {
            settings: AdminSettings::from(&settings),
        })
        .ok()
    }
}

mod put {
    use super::AdminSettings;
    use crate::settings::ExtensionSettingsData;
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        modrinth_enabled: bool,
        #[garde(skip)]
        curseforge_enabled: bool,
        /// Omit to keep the stored key, send an empty string to remove it.
        #[garde(length(chars, max = 255))]
        #[schema(max_length = 255)]
        curseforge_api_key: Option<String>,
        #[garde(skip)]
        allow_clean_install: bool,
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        installer_image: String,
        #[garde(length(max = 200), inner(length(chars, min = 1, max = 128)))]
        excluded_mods: Vec<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        settings: AdminSettings,
    }

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("modpacks.manage")?;

        let installer_image = data.installer_image.trim().to_string();
        if installer_image.chars().any(char::is_whitespace) {
            return ApiResponse::error("the installer image must not contain whitespace")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        let key_changed = data.curseforge_api_key.is_some();
        let mut settings = state.settings.get_mut().await?;
        let extension = settings.find_mut_extension_settings::<ExtensionSettingsData>()?;
        extension.modrinth_enabled = data.modrinth_enabled;
        extension.curseforge_enabled = data.curseforge_enabled;
        if let Some(key) = data.curseforge_api_key {
            // a pasted key with a trailing newline is not a valid header value
            let key = key.trim();
            extension.curseforge_api_key = (!key.is_empty()).then(|| key.to_string());
        }
        extension.allow_clean_install = data.allow_clean_install;
        extension.installer_image = installer_image;
        extension.excluded_mods = data
            .excluded_mods
            .iter()
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .collect();
        let response = AdminSettings::from(&*extension);
        settings.save().await?;

        activity_logger
            .log(
                "modpacks:settings.update",
                serde_json::json!({
                    "modrinth_enabled": response.modrinth_enabled,
                    "curseforge_enabled": response.curseforge_enabled,
                    "curseforge_api_key_changed": key_changed,
                    "allow_clean_install": response.allow_clean_install,
                    "installer_image": response.installer_image,
                    "excluded_mods": response.excluded_mods,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response { settings: response }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(put::route))
        .with_state(state.clone())
}
