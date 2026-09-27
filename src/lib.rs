use indexmap::IndexMap;
use shared::{
    State,
    extensions::{
        Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder,
        settings::ExtensionSettingsDeserializer,
    },
    permissions::PermissionGroup,
};
use std::sync::Arc;

mod detect;
mod install;
mod providers;
mod routes;
mod settings;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| {
                router.nest(
                    "/extensions/dev.caloptreyx.modpacks",
                    routes::admin::router(&state),
                )
            })
            .add_client_server_api_router(|router| {
                router.nest("/modpacks", routes::server::router(&state))
            })
    }

    async fn initialize_permissions(
        &mut self,
        _state: State,
        mut builder: ExtensionPermissionsBuilder,
    ) -> ExtensionPermissionsBuilder {
        builder.server_permissions.insert(
            "modpacks",
            PermissionGroup {
                description: "Permissions that control the ability to browse and install Minecraft modpacks.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows browsing modpacks and viewing the detected mod loader, Minecraft version and installed modpack.",
                    ),
                    (
                        "install",
                        "Allows installing modpacks. This reinstalls the server and replaces its mods, mod loader and (optionally) all files.",
                    ),
                ]),
            },
        );

        builder.admin_permissions.insert(
            "modpacks",
            PermissionGroup {
                description: "Permissions that control the ability to configure the modpack installer extension.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows viewing the modpack installer settings.",
                    ),
                    (
                        "manage",
                        "Allows changing the modpack installer settings, including the CurseForge API key.",
                    ),
                ]),
            },
        );

        builder
    }

    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(settings::ExtensionSettingsDataDeserializer)
    }
}
