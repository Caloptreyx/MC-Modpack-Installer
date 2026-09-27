# Modpack Installer

A [Calagopus Panel](https://calagopus.com) extension that lets users browse Minecraft modpacks on
**Modrinth** and **CurseForge** and install them onto their server in one click.

Package name: `dev.caloptreyx.modpacks` · Requires panel `>=1.2.2`

## Features

- **Browse Modpacks page**: a card grid with icons, authors, loaders, supported Minecraft versions,
  categories, download and follow counts. It has search, sorting (relevance, downloads, follows,
  newest, recently updated), filters for mod loader, Minecraft version and category, and a
  *Match this server* toggle that applies the detected loader and version. Modrinth packs marked
  as unsupported on servers are hidden by default. Filters and the page number are kept in the
  URL, so going back from a modpack restores the list.
- **Modpack Details page**: a header with stats, links and categories, plus tabs for the
  description (Modrinth markdown / CurseForge HTML), the versions and the gallery.
- **Versions tab**: a paginated table of every version with its release type, Minecraft versions,
  loaders, publish date, downloads and size. You can search it, filter by loader, Minecraft
  version and release type, and toggle *Compatible with this server*. Each row opens the
  changelog or installs that version.
- **Install Modpack modal**: pick the mod loader, the Minecraft version and the modpack version.
  The loader and Minecraft version default to what was detected on the server, and each list
  only offers combinations the modpack actually ships. The modal warns when the loader or
  Minecraft version will change and when the pack is client-only.
- **Java version selection**: the modal recommends the egg docker image that provides the Java
  version the chosen Minecraft version needs (8 / 17 / 21 / 25) and can switch to it. This
  requires the `startup.docker-image` permission.
- **Installed modpack card**: shows the modpack, version, loader and install date, and checks for
  updates for the installed loader and Minecraft version. It offers one-click update or reinstall.
- **Detection** of the mod loader and Minecraft version. Sources, in order:
  1. the install marker (`.modpack-installer.json`)
  2. loader files (`libraries/net/neoforged`, `libraries/net/minecraftforge`, `.fabric`, `.quilt`, …)
  3. the [MC Version Chooser](https://github.com/Regrave/mc-version-chooser) marker
  4. `logs/latest.log`
  5. the egg name and variables (`MINECRAFT_VERSION`, `FORGE_VERSION`, …)

### Installation

Installs run through the panel's native Wings reinstall flow. The server locks into the
*installing* state, the console shows live progress, and the stock *Cancel* button works.

- **Keep worlds & settings** (default) replaces `mods/`, the mod loader and the pack's configs. It
  keeps worlds, `server.properties`, whitelists, ops and bans. Protected files are only created
  when they do not exist yet.
- **Clean install** deletes every server file first. An administrator can disable it.
- Every file is downloaded and verified (SHA-512/SHA-1) into a staging directory before anything
  on the server is touched, so a failed download leaves the server as it was.
- Mod loaders (Fabric, Quilt, Forge, NeoForge) are installed from
  [mcjars](https://versions.mcjars.app). If mcjars lacks the exact loader version, the extension
  falls back to the official Forge/NeoForge installer (with a temporary Java runtime) or to Fabric
  meta.
- Client-only mods are removed. This covers files marked server-unsupported on Modrinth,
  CurseForge files tagged *Client* only, mods whose `fabric.mod.json` / `quilt.mod.json` declares
  `client`, a list of well-known client-only mods, and the administrator's own list. A mod is kept
  when another mod depends on it.
- CurseForge files that block third-party downloads are replaced by the identical Modrinth file
  (matched by SHA-1) when one exists. Otherwise they are listed in `MODPACK-MISSING-FILES.txt` and
  on the installed modpack card.
- EULA acceptance is optional (`eula.txt`).

## Installing the extension

Download `dev_caloptreyx_modpacks.c7s.zip` from the
[latest release](https://github.com/Caloptreyx/MC-Modpack-Installer/releases/latest). Then either
upload it under **Admin → Extensions**, or drop it into your heavy image's `build/extensions/`
directory and run `docker compose restart web`. Extensions require the `:heavy` panel image (or a
dev environment); see the
[Calagopus docs](https://calagopus.com/docs/panel/extensions/installing-extensions).

## Configuration

**Admin → Extensions → Modpack Installer → Configure**

- **Platforms**: enable Modrinth and/or CurseForge.
  - CurseForge needs an API key from [console.curseforge.com](https://console.curseforge.com/).
  - The key is stored encrypted, never sent to users, and can be tested before saving.
- **Installation**:
  - allow or forbid clean installs
  - the installer docker image (default `python:3.13-slim`; any glibc image with Python 3.12+)
  - additional client-only mods to remove, as mod ids or filename globs

Permissions:

- server: `modpacks.read` (browse), `modpacks.install` (install)
- admin: `modpacks.read`, `modpacks.manage`

Switching the docker image while installing also requires the core `startup.docker-image`
permission.

## API

- `GET /api/client/servers/{server}/modpacks`: platforms, detected loader and version, installed
  modpack, docker images
- `GET .../modpacks/search?provider&page&per_page&search&loader&game_version&category&sort&hide_client_only`
- `GET .../modpacks/filters?provider`
- `GET .../modpacks/projects/{provider}/{project}`
- `GET .../modpacks/projects/{provider}/{project}/versions?page&per_page&search&loader&game_version&release_type`
- `GET .../modpacks/projects/{provider}/{project}/versions/{version}` (includes the changelog)
- `POST .../modpacks/install` `{ provider, project_id, version_id, mode: replace|wipe, accept_eula, docker_image? }`
- `GET|PUT /api/admin/extensions/dev.caloptreyx.modpacks/settings`
- `POST /api/admin/extensions/dev.caloptreyx.modpacks/curseforge/verify` `{ api_key? }`

Download URLs are always resolved on the backend. The install script only fetches pack files from
the Modrinth CDN, the modrinth.index.json host allowlist and the CurseForge CDN. Modpack
installations are logged as the `server:modpacks.install` activity.

## License

MIT
