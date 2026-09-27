use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};

pub const DEFAULT_INSTALLER_IMAGE: &str = "python:3.13-slim";

#[derive(Clone)]
pub struct ExtensionSettingsData {
    /// Whether modpacks can be browsed and installed from Modrinth.
    pub modrinth_enabled: bool,
    /// Whether modpacks can be browsed and installed from CurseForge (requires an API key).
    pub curseforge_enabled: bool,
    /// CurseForge Core API key, stored encrypted and never sent to clients.
    pub curseforge_api_key: Option<String>,
    /// Whether users may wipe all server files when installing a modpack.
    pub allow_clean_install: bool,
    /// Docker image the installation script runs in. Needs `python3` (3.12+) and glibc.
    pub installer_image: String,
    /// Additional client-only mods removed after installing, as mod ids or
    /// case-insensitive filename globs (e.g. `oculus`, `*-client-*.jar`).
    pub excluded_mods: Vec<String>,
}

impl Default for ExtensionSettingsData {
    fn default() -> Self {
        Self {
            modrinth_enabled: true,
            curseforge_enabled: true,
            curseforge_api_key: None,
            allow_clean_install: true,
            installer_image: DEFAULT_INSTALLER_IMAGE.to_string(),
            excluded_mods: Vec::new(),
        }
    }
}

impl ExtensionSettingsData {
    /// The CurseForge API key when CurseForge is enabled and configured.
    pub fn curseforge_key(&self) -> Option<&str> {
        self.curseforge_api_key
            .as_deref()
            .filter(|key| self.curseforge_enabled && !key.is_empty())
    }
}

#[async_trait::async_trait]
impl SettingsSerializeExt for ExtensionSettingsData {
    async fn serialize(
        &self,
        serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        let serializer = serializer
            .write_serde_setting("modrinth_enabled", &self.modrinth_enabled)?
            .write_serde_setting("curseforge_enabled", &self.curseforge_enabled)?
            .write_serde_setting("allow_clean_install", &self.allow_clean_install)?
            .write_raw_setting("installer_image", &*self.installer_image)
            .write_serde_setting("excluded_mods", &self.excluded_mods)?;

        Ok(
            match self.curseforge_api_key.as_deref().filter(|key| !key.is_empty()) {
                Some(key) => {
                    serializer
                        .write_raw_encrypted_setting("curseforge_api_key", key)
                        .await?
                }
                None => serializer.write_raw_setting("curseforge_api_key", ""),
            },
        )
    }
}

pub struct ExtensionSettingsDataDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for ExtensionSettingsDataDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let defaults = ExtensionSettingsData::default();

        let curseforge_api_key = match deserializer.read_raw_setting("curseforge_api_key") {
            Some(value) if !value.is_empty() => Some(
                deserializer
                    .database
                    .decrypt_base64(value)
                    .await?
                    .to_string(),
            ),
            _ => None,
        };

        Ok(Box::new(ExtensionSettingsData {
            modrinth_enabled: deserializer
                .read_serde_setting("modrinth_enabled")
                .unwrap_or(defaults.modrinth_enabled),
            curseforge_enabled: deserializer
                .read_serde_setting("curseforge_enabled")
                .unwrap_or(defaults.curseforge_enabled),
            curseforge_api_key,
            allow_clean_install: deserializer
                .read_serde_setting("allow_clean_install")
                .unwrap_or(defaults.allow_clean_install),
            installer_image: deserializer
                .read_raw_setting("installer_image")
                .filter(|image| !image.is_empty())
                .map(|image| image.to_string())
                .unwrap_or(defaults.installer_image),
            excluded_mods: deserializer
                .read_serde_setting("excluded_mods")
                .unwrap_or(defaults.excluded_mods),
        }))
    }
}

/// Loads a copy of the extension settings.
pub async fn load(state: &shared::State) -> Result<ExtensionSettingsData, anyhow::Error> {
    Ok(state
        .settings
        .get()
        .await?
        .find_extension_settings::<ExtensionSettingsData>()?
        .clone())
}
