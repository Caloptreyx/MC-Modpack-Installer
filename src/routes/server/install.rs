use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use crate::{
        install::{InstallMode, ScriptOptions},
        providers::{Client, Provider},
    };
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            UpdatableModel,
            server::{GetServer, GetServerActivityLogger, UpdateServerOptions},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        provider: Provider,
        #[garde(length(chars, min = 1, max = 64))]
        #[schema(min_length = 1, max_length = 64)]
        project_id: String,
        #[garde(length(chars, min = 1, max = 64))]
        #[schema(min_length = 1, max_length = 64)]
        version_id: String,
        #[garde(skip)]
        mode: InstallMode,
        /// Writes `eula=true` to eula.txt.
        #[garde(skip)]
        #[serde(default)]
        accept_eula: bool,
        /// Switches the server to this docker image of its egg before installing.
        #[garde(length(chars, min = 2, max = 255))]
        #[schema(min_length = 2, max_length = 255)]
        docker_image: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(post, path = "/", responses(
        (status = ACCEPTED, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = FORBIDDEN, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = EXPECTATION_FAILED, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("modpacks.install")?;

        if server.status.is_some() {
            return ApiResponse::error("the server is already installing or restoring a backup")
                .with_status(StatusCode::CONFLICT)
                .ok();
        }

        let settings = crate::settings::load(&state).await?;
        if data.mode == InstallMode::Wipe && !settings.allow_clean_install {
            return ApiResponse::error("clean installs have been disabled by an administrator")
                .with_status(StatusCode::FORBIDDEN)
                .ok();
        }

        let client = Client::new(&settings, data.provider)?;
        let project = client.project(&state, &data.project_id).await?;
        let pack = client.pack_file(&project, &data.version_id).await?;

        let docker_image = data
            .docker_image
            .map(|image| image.trim().to_string())
            .filter(|image| *image != server.image);
        if let Some(image) = &docker_image {
            permissions.has_server_permission("startup.docker-image")?;

            if !server.egg.docker_images.values().any(|egg_image| egg_image == image) {
                return ApiResponse::error("the specified docker image is not available")
                    .with_status(StatusCode::EXPECTATION_FAILED)
                    .ok();
            }
            if !state.settings.get().await?.server.allow_overwriting_custom_docker_image
                && !server.egg.docker_images.values().any(|egg_image| *egg_image == server.image)
            {
                return ApiResponse::error("overwriting custom docker images is not allowed")
                    .with_status(StatusCode::EXPECTATION_FAILED)
                    .ok();
            }
        }

        let script = crate::install::script(ScriptOptions {
            provider: data.provider,
            project: &project,
            version_id: &data.version_id,
            pack: &pack,
            mode: data.mode,
            accept_eula: data.accept_eula,
            curseforge_key: client.curseforge_key(),
            excluded_mods: &settings.excluded_mods,
            image: &settings.installer_image,
        });

        // Run to completion even if the client disconnects: the install
        // commits the server status and starts the Wings reinstall together.
        tokio::spawn(async move {
            if let Some(image) = &docker_image {
                server
                    .update(
                        &state,
                        UpdateServerOptions {
                            image: Some(image.as_str().into()),
                            ..Default::default()
                        },
                    )
                    .await?;
            }

            server
                .install(&state, data.mode == InstallMode::Wipe, Some(script))
                .await?;

            activity_logger
                .log(
                    "server:modpacks.install",
                    serde_json::json!({
                        "provider": data.provider,
                        "project_id": project.id,
                        "version_id": data.version_id,
                        "name": project.name,
                        "version": pack.version_name,
                        "mode": data.mode,
                        "accept_eula": data.accept_eula,
                        "docker_image": docker_image,
                    }),
                )
                .await;

            if docker_image.is_some() {
                server.0.batch_sync(&state.database).await;
            }

            ApiResponse::new_serialized(Response {})
                .with_status(StatusCode::ACCEPTED)
                .ok()
        })
        .await?
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
