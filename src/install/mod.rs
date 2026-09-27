//! Builds the Wings installation script that installs a modpack.

use crate::providers::{ModpackDetails, PackFile, Provider};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use utoipa::ToSchema;
use wings_api::InstallationScript;

/// The installer, executed with the `python3` of the installer image.
const INSTALLER: &str = include_str!("install.py");
const HEREDOC_DELIMITER: &str = "MPI_INSTALLER_PY_EOF";

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InstallMode {
    /// Keep worlds and settings, replace the mods, scripts and mod loader.
    Replace,
    /// Delete every server file before installing.
    Wipe,
}

pub struct ScriptOptions<'a> {
    pub provider: Provider,
    pub project: &'a ModpackDetails,
    pub version_id: &'a str,
    pub pack: &'a PackFile,
    pub mode: InstallMode,
    pub accept_eula: bool,
    pub curseforge_key: Option<&'a str>,
    pub excluded_mods: &'a [String],
    pub image: &'a str,
}

fn wrapper() -> String {
    format!(
        "#!/bin/bash\nset -e\ncat > /tmp/modpack-installer.py <<'{HEREDOC_DELIMITER}'\n{INSTALLER}\n{HEREDOC_DELIMITER}\nexec python3 /tmp/modpack-installer.py\n"
    )
}

pub fn script(options: ScriptOptions<'_>) -> InstallationScript {
    let mut environment = indexmap::IndexMap::new();
    let mut set = |key: &str, value: &str| {
        environment.insert(
            compact_str::CompactString::from(key),
            serde_json::Value::String(value.to_string()),
        );
    };

    set("MPI_SOURCE", options.provider.as_str());
    set("MPI_PACK_URL", &options.pack.url);
    set("MPI_PACK_SHA1", options.pack.sha1.as_deref().unwrap_or_default());
    set("MPI_PROJECT_ID", &options.project.id);
    set("MPI_VERSION_ID", options.version_id);
    set("MPI_PACK_NAME", &options.project.name);
    set("MPI_VERSION_NAME", &options.pack.version_name);
    set("MPI_ICON_URL", options.project.icon_url.as_deref().unwrap_or_default());
    set(
        "MPI_INSTALL_MODE",
        match options.mode {
            InstallMode::Replace => "replace",
            InstallMode::Wipe => "wipe",
        },
    );
    set("MPI_ACCEPT_EULA", if options.accept_eula { "1" } else { "0" });
    set("MPI_EXCLUDE", &options.excluded_mods.join("\n"));
    set(
        "MPI_USER_AGENT",
        concat!(
            "Caloptreyx/MC-Modpack-Installer/",
            env!("CARGO_PKG_VERSION"),
            " (Calagopus extension dev.caloptreyx.modpacks)"
        ),
    );
    if let Some(key) = options.curseforge_key {
        set("MPI_CF_API_KEY", key);
    }

    InstallationScript {
        container_image: options.image.into(),
        entrypoint: "/bin/bash".into(),
        script: wrapper().into(),
        environment,
    }
}

#[derive(ToSchema, Serialize, Clone)]
pub struct DockerImage {
    pub name: String,
    pub image: String,
    /// Java feature release the image provides, parsed from its name or tag.
    pub java_version: Option<u32>,
}

/// Parses the Java version out of an egg docker image name or tag
/// (`Java 21`, `ghcr.io/ptero-eggs/yolks:java_21`, `eclipse-temurin:17-jre`).
pub fn java_version_of(value: &str) -> Option<u32> {
    static JAVA: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(?:java|jdk|jre|temurin|openjdk|zulu|graalvm|corretto)\D{0,3}?(\d{1,2})(?:\D|$)")
            .expect("invalid java regex")
    });
    JAVA.captures(value)
        .and_then(|captures| captures[1].parse().ok())
        .filter(|version| (7..=40).contains(version))
}

pub fn docker_images<'a, I>(images: I) -> Vec<DockerImage>
where
    I: IntoIterator<Item = (&'a compact_str::CompactString, &'a compact_str::CompactString)>,
{
    images
        .into_iter()
        .map(|(name, image)| DockerImage {
            name: name.to_string(),
            image: image.to_string(),
            java_version: java_version_of(name).or_else(|| java_version_of(image)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_cannot_terminate_the_heredoc_early() {
        assert!(!INSTALLER.lines().any(|line| line.trim() == HEREDOC_DELIMITER));
        assert!(wrapper().ends_with("exec python3 /tmp/modpack-installer.py\n"));
    }

    #[test]
    fn java_versions_are_parsed_from_image_names_and_tags() {
        assert_eq!(java_version_of("Java 21"), Some(21));
        assert_eq!(java_version_of("ghcr.io/ptero-eggs/yolks:java_8"), Some(8));
        assert_eq!(java_version_of("eclipse-temurin:17-jre"), Some(17));
        assert_eq!(java_version_of("ghcr.io/ptero-eggs/yolks:java_25"), Some(25));
        assert_eq!(java_version_of("ghcr.io/ptero-eggs/yolks:debian"), None);
    }
}
