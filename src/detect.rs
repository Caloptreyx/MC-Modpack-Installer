//! Detects the mod loader, Minecraft version and installed modpack of a server
//! from its files (through Wings) and its egg.

use crate::providers::{Loader, Provider, compare_game_versions, is_release_version};
use serde::{Deserialize, Serialize};
use shared::{
    State,
    models::{server::Server, server_variable::ServerVariable},
};
use std::sync::LazyLock;
use tokio::io::AsyncReadExt;
use utoipa::ToSchema;
use wings_api::client::WingsClient;

/// File written by the installation script, see `install.py`.
pub const MARKER_FILE: &str = ".modpack-installer.json";
const MAX_MARKER_SIZE: u64 = 64 * 1024;
const MAX_LOG_SIZE: u64 = 256 * 1024;

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DetectionSource {
    /// Read from the marker of a modpack installed by this extension.
    Modpack,
    /// Derived from mod loader files on the server.
    Files,
    /// Derived from the server's egg and its variables.
    Egg,
}

#[derive(ToSchema, Serialize, Default, Debug)]
pub struct Detection {
    pub loader: Option<Loader>,
    pub minecraft_version: Option<String>,
    pub source: Option<DetectionSource>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct InstalledModpack {
    #[serde(rename = "source")]
    pub provider: Provider,
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    #[serde(default)]
    pub version_name: String,
    pub icon_url: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<Loader>,
    pub loader_version: Option<String>,
    pub installed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub removed_client_mods: Vec<String>,
    #[serde(default)]
    pub missing_files: Vec<String>,
}

/// The marker of MC Version Chooser (gg.ir77.mcversionchooser), read for interoperability.
#[derive(Deserialize)]
struct VersionChooserMarker {
    #[serde(rename = "type")]
    kind: Option<String>,
    version: Option<String>,
}

async fn read_file(client: &WingsClient, server: uuid::Uuid, path: &str, limit: u64) -> Option<Vec<u8>> {
    let reader = client
        .get_servers_server_files_contents(
            server,
            &wings_api::servers_server_files_contents::get::Query {
                file: Some(path.into()),
                max_size: Some(limit),
                ..Default::default()
            },
        )
        .await
        .ok()?;

    let mut content = Vec::new();
    reader.take(limit).read_to_end(&mut content).await.ok()?;
    Some(content)
}

async fn list_directory(client: &WingsClient, server: uuid::Uuid, directory: &str) -> Vec<(String, bool)> {
    client
        .get_servers_server_files_list_directory(
            server,
            &wings_api::servers_server_files_list_directory::get::Query {
                directory: Some(directory.into()),
                ..Default::default()
            },
        )
        .await
        .map(|entries| {
            entries
                .into_iter()
                .map(|entry| (entry.name.to_string(), entry.directory))
                .collect()
        })
        .unwrap_or_default()
}

fn newest_directory(entries: &[(String, bool)]) -> Option<&str> {
    entries
        .iter()
        .filter(|(_, directory)| *directory)
        .map(|(name, _)| name.as_str())
        .max_by(|a, b| compare_game_versions(a, b))
}

/// Maps a NeoForge version to its Minecraft release: `21.1.77` -> `1.21.1`,
/// `20.4.237` -> `1.20.4`, `21.0.x` -> `1.21`, `26.1.0.5` -> `26.1`.
pub fn neoforge_minecraft_version(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    if major >= 25 {
        return Some(if minor == 0 { format!("{major}") } else { format!("{major}.{minor}") });
    }
    Some(if minor == 0 {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    })
}

/// Detects the loader from mod loader files, returning the Minecraft version when the files encode it.
async fn detect_from_files(client: &WingsClient, server: uuid::Uuid) -> Option<(Loader, Option<String>)> {
    let root = list_directory(client, server, "/").await;
    let has = |name: &str| root.iter().any(|(entry, _)| entry == name);

    if has("libraries") {
        let neoforge = list_directory(client, server, "/libraries/net/neoforged/neoforge").await;
        if let Some(version) = newest_directory(&neoforge) {
            return Some((Loader::Neoforge, neoforge_minecraft_version(version)));
        }
        // NeoForge for 1.20.1 still uses the forge artifact under net/neoforged.
        let neoforged_forge = list_directory(client, server, "/libraries/net/neoforged/forge").await;
        if let Some(version) = newest_directory(&neoforged_forge) {
            return Some((Loader::Neoforge, version.split('-').next().map(str::to_string)));
        }
        let forge = list_directory(client, server, "/libraries/net/minecraftforge/forge").await;
        if let Some(version) = newest_directory(&forge) {
            return Some((Loader::Forge, version.split('-').next().map(str::to_string)));
        }
    }
    if has(".fabric") || has("fabric-server-launch.jar") || has("fabric-server-launcher.properties") {
        return Some((Loader::Fabric, None));
    }
    if has(".quilt") || has("quilt-server-launch.jar") {
        return Some((Loader::Quilt, None));
    }

    if has(".mcvc-type.json")
        && let Some(content) = read_file(client, server, ".mcvc-type.json", MAX_MARKER_SIZE).await
        && let Ok(marker) = serde_json::from_slice::<VersionChooserMarker>(&content)
        && let Some(loader) = marker.kind.as_deref().and_then(Loader::parse)
    {
        return Some((loader, marker.version.filter(|version| is_release_version(version))));
    }

    None
}

/// First Minecraft release named by a startup log line (vanilla/Forge/NeoForge, Fabric/Quilt, Paper-family).
fn log_minecraft_version(content: &str) -> Option<String> {
    static STARTING: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?:Starting minecraft server version|Loading Minecraft|for Minecraft) (\S+)")
            .expect("invalid log regex")
    });

    STARTING
        .captures_iter(content)
        .map(|captures| captures[1].to_string())
        .find(|version| is_release_version(version))
}

async fn version_from_log(client: &WingsClient, server: uuid::Uuid) -> Option<String> {
    let content = read_file(client, server, "logs/latest.log", MAX_LOG_SIZE).await?;
    log_minecraft_version(&String::from_utf8_lossy(&content))
}

/// Loader and Minecraft version hinted by the egg name and variables.
async fn detect_from_egg(state: &State, server: &Server) -> (Option<Loader>, Option<String>) {
    let name = server.egg.name.to_ascii_lowercase();
    let mut loader = if name.contains("neoforge") {
        Some(Loader::Neoforge)
    } else if name.contains("forge") {
        Some(Loader::Forge)
    } else if name.contains("fabric") {
        Some(Loader::Fabric)
    } else if name.contains("quilt") {
        Some(Loader::Quilt)
    } else {
        None
    };

    let mut minecraft_version = None;
    if let Ok(variables) =
        ServerVariable::all_by_server_uuid_egg_uuid(&state.database, server.uuid, server.egg.uuid).await
    {
        for variable in variables {
            let value = variable.value.trim();
            match variable.variable.env_variable.as_str() {
                "MINECRAFT_VERSION" | "MC_VERSION" | "VANILLA_VERSION" | "VERSION" | "GAME_VERSION"
                    if minecraft_version.is_none() && is_release_version(value) =>
                {
                    minecraft_version = Some(value.to_string());
                }
                "NEOFORGE_VERSION" if loader.is_none() && !value.is_empty() => loader = Some(Loader::Neoforge),
                "FORGE_VERSION" if loader.is_none() && !value.is_empty() => loader = Some(Loader::Forge),
                "FABRIC_VERSION" | "FABRIC_LOADER_VERSION" if loader.is_none() && !value.is_empty() => {
                    loader = Some(Loader::Fabric)
                }
                "QUILT_VERSION" | "QUILT_LOADER_VERSION" if loader.is_none() && !value.is_empty() => {
                    loader = Some(Loader::Quilt)
                }
                _ => {}
            }
        }
    }

    (loader, minecraft_version)
}

/// Detects the server's setup. Failures to reach Wings only reduce what can be detected.
pub async fn detect(state: &State, server: &Server) -> (Detection, Option<InstalledModpack>) {
    let client = match server.node.fetch_cached(&state.database).await {
        Ok(node) => node.api_client(&state.database).await.ok(),
        Err(_) => None,
    };

    let mut installed = None;
    let mut detection = Detection::default();

    if let Some(client) = &client {
        installed = read_file(client, server.uuid, MARKER_FILE, MAX_MARKER_SIZE)
            .await
            .and_then(|content| serde_json::from_slice::<InstalledModpack>(&content).ok());

        if let Some(modpack) = &installed
            && modpack.loader.is_some()
        {
            detection = Detection {
                loader: modpack.loader,
                minecraft_version: modpack.minecraft_version.clone(),
                source: Some(DetectionSource::Modpack),
            };
        } else if let Some((loader, version)) = detect_from_files(client, server.uuid).await {
            detection = Detection {
                loader: Some(loader),
                minecraft_version: version,
                source: Some(DetectionSource::Files),
            };
        }
    }

    if detection.loader.is_none() || detection.minecraft_version.is_none() {
        let (egg_loader, egg_version) = detect_from_egg(state, server).await;
        if detection.loader.is_none() && egg_loader.is_some() {
            detection.loader = egg_loader;
            detection.source = Some(DetectionSource::Egg);
        }
        if detection.minecraft_version.is_none() {
            detection.minecraft_version = match &client {
                Some(client) => version_from_log(client, server.uuid).await,
                None => None,
            }
            .or(egg_version);
        }
        if detection.source.is_none() && detection.minecraft_version.is_some() {
            detection.source = Some(DetectionSource::Egg);
        }
    }

    (detection, installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_versions_map_to_minecraft_releases() {
        assert_eq!(neoforge_minecraft_version("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_minecraft_version("20.4.237").as_deref(), Some("1.20.4"));
        assert_eq!(neoforge_minecraft_version("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(neoforge_minecraft_version("26.1.0.12-beta").as_deref(), Some("26.1"));
        assert_eq!(neoforge_minecraft_version("snapshot"), None);
    }

    #[test]
    fn minecraft_versions_are_found_in_startup_logs() {
        for (line, version) in [
            ("[Server thread/INFO]: Starting minecraft server version 1.20.1", "1.20.1"),
            ("[main/INFO]: Loading Minecraft 1.21.1 with Fabric Loader 0.16.10", "1.21.1"),
            ("[bootstrap] Loading Paper 26.3-32-dev/26.3@0c803ba (2026-09-21) for Minecraft 26.3", "26.3"),
        ] {
            assert_eq!(log_minecraft_version(line).as_deref(), Some(version), "{line}");
        }
    }

    #[test]
    fn installed_marker_parses() {
        let marker: InstalledModpack = serde_json::from_str(
            r#"{"source":"modrinth","project_id":"8Gbixvts","version_id":"nW5BtnR5","name":"Pack",
            "version_name":"5.0.5","icon_url":null,"minecraft_version":"1.21.11","loader":"fabric",
            "loader_version":"0.19.3","installed_at":"2026-09-27T18:26:11.221390+00:00",
            "managed":["mods"],"removed_client_mods":["a.jar"],"missing_files":[]}"#,
        )
        .unwrap();
        assert_eq!(marker.provider, Provider::Modrinth);
        assert_eq!(marker.loader, Some(Loader::Fabric));
        assert_eq!(marker.removed_client_mods, ["a.jar"]);
    }
}
