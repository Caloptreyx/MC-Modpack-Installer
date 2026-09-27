"""Modpack installer for the dev.caloptreyx.modpacks Calagopus extension.

Runs inside the Wings installation container (server files mounted at /mnt/server).
Every input comes from `MPI_*` environment variables set by the extension backend;
the server's own environment (SERVER_JARFILE, ...) and the Wings install status /
progress files are available as well.

Flow: download + verify the pack archive, resolve and stage every file in a
temporary directory (nothing on the server is touched until all downloads
succeeded), then replace the previous mods and loader, apply overrides, install
the mod loader, drop client-only mods and write `.modpack-installer.json`.
"""

import concurrent.futures
import datetime
import fnmatch
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(os.environ.get("MPI_ROOT", "/mnt/server"))
TMP = ROOT / ".modpack-installer-tmp"
MARKER = ROOT / ".modpack-installer.json"
MISSING_FILE = ROOT / "MODPACK-MISSING-FILES.txt"

SOURCE = os.environ.get("MPI_SOURCE", "")
PACK_URL = os.environ.get("MPI_PACK_URL", "")
PACK_SHA1 = os.environ.get("MPI_PACK_SHA1", "").lower()
PROJECT_ID = os.environ.get("MPI_PROJECT_ID", "")
VERSION_ID = os.environ.get("MPI_VERSION_ID", "")
PACK_NAME = os.environ.get("MPI_PACK_NAME", "Modpack")
VERSION_NAME = os.environ.get("MPI_VERSION_NAME", "")
ICON_URL = os.environ.get("MPI_ICON_URL", "")
INSTALL_MODE = os.environ.get("MPI_INSTALL_MODE", "replace")
ACCEPT_EULA = os.environ.get("MPI_ACCEPT_EULA", "0") == "1"
CF_API_KEY = os.environ.get("MPI_CF_API_KEY", "")
EXCLUDE_PATTERNS = [
    line.strip().lower()
    for line in os.environ.get("MPI_EXCLUDE", "").splitlines()
    if line.strip() and not line.strip().startswith("#")
]
USER_AGENT = os.environ.get("MPI_USER_AGENT", "Caloptreyx/MC-Modpack-Installer")
SERVER_JARFILE = os.environ.get("SERVER_JARFILE", "").strip() or "server.jar"
PROGRESS_FILE = os.environ.get("INSTALL_PROGRESS_FILE", "")
STATUS_FILE = os.environ.get("INSTALL_STATUS_FILE", "")
WORKERS = 8

CURSEFORGE_API = "https://api.curseforge.com"
MODRINTH_API = "https://api.modrinth.com/v2"
MCJARS_API = "https://versions.mcjars.app/api/v2"

# Download hosts allowed inside a .mrpack (Modrinth modpack format specification).
MRPACK_HOSTS = ("cdn.modrinth.com", "github.com", "raw.githubusercontent.com", "gitlab.com")
CURSEFORGE_HOSTS = ("edge.forgecdn.net", "mediafilez.forgecdn.net", "media.forgecdn.net")

# Top-level paths that belong to the server operator. Overrides only create them
# when they do not exist yet, so a pack can ship a starting world or
# server.properties without ever overwriting the ones already in use.
PROTECTED = {
    "server.properties",
    "eula.txt",
    "ops.json",
    "whitelist.json",
    "banned-ips.json",
    "banned-players.json",
    "usercache.json",
    "world",
    "world_nether",
    "world_the_end",
}

# Override entries that only matter to a game client.
CLIENT_OVERRIDES = {
    "resourcepacks",
    "shaderpacks",
    "screenshots",
    "saves",
    "logs",
    "crash-reports",
    "options.txt",
    "optionsof.txt",
    "optionsshaders.txt",
    "servers.dat",
    "servers.dat_old",
}

# Pack-owned directories that are removed before an update when the previous
# install came from a pack as well (stale scripts would otherwise keep running).
REPLACEABLE_SCRIPT_DIRS = ("kubejs", "scripts")

# Well-known client-only mods (mod ids) that crash or misbehave on dedicated
# servers and do not declare themselves as client-only in their metadata.
KNOWN_CLIENT_ONLY = {
    "betterf3",
    "chat_heads",
    "cherishedworlds",
    "citresewn",
    "controlling",
    "ding",
    "drippyloadingscreen",
    "dynamiclightsreforged",
    "embeddium",
    "embeddiumplus",
    "entityculling",
    "fancymenu",
    "fpsreducer",
    "immediatelyfast",
    "iris",
    "legendarytooltips",
    "magnesium",
    "moreculling",
    "mousetweaks",
    "notenoughanimations",
    "oculus",
    "optifine",
    "reeses_sodium_options",
    "rubidium",
    "rubidium_extra",
    "skinlayers3d",
    "sodium",
    "sodiumextra",
    "toastcontrol",
}

RETRYABLE_STATUS = {408, 425, 429, 500, 502, 503, 504}

# CurseForge class ids.
CF_CLASS_MODS = 6
CF_CLASS_RESOURCE_PACKS = 12
CF_CLASS_WORLDS = 17
CF_CLASS_SHADERS = 6552
CF_CLASS_DATAPACKS = 6945


class InstallError(Exception):
    pass


def log(message):
    print(f"[modpack-installer] {message}", flush=True)


def progress(percent, label):
    if not PROGRESS_FILE:
        return
    try:
        with open(PROGRESS_FILE, "w", encoding="utf-8") as handle:
            handle.write(f"{max(0, min(100, int(percent)))}/100 {label}\n")
    except OSError:
        pass


def report_failure(reason):
    if not STATUS_FILE:
        return
    try:
        with open(STATUS_FILE, "w", encoding="utf-8") as handle:
            handle.write(reason.replace("\n", " ")[:250] + "\n")
    except OSError:
        pass


def host_of(url):
    return (urllib.parse.urlparse(url).hostname or "").lower()


def host_allowed(url, hosts):
    parsed = urllib.parse.urlparse(url)
    host = (parsed.hostname or "").lower()
    return parsed.scheme == "https" and any(host == h or host.endswith("." + h) for h in hosts)


def request(url, data=None, headers=None, method=None, timeout=120):
    """Performs an HTTP request with retries on transient failures and returns the response body."""
    all_headers = {"User-Agent": USER_AGENT, "Accept": "application/json"}
    all_headers.update(headers or {})
    body = None
    if data is not None:
        body = json.dumps(data).encode("utf-8")
        all_headers["Content-Type"] = "application/json"

    last_error = None
    for attempt in range(1, 7):
        try:
            req = urllib.request.Request(url, data=body, headers=all_headers, method=method)
            with urllib.request.urlopen(req, timeout=timeout) as response:
                return response.read()
        except urllib.error.HTTPError as err:
            last_error = err
            if err.code not in RETRYABLE_STATUS:
                raise InstallError(f"{method or ('POST' if body else 'GET')} {url} failed with HTTP {err.code}") from err
        except (urllib.error.URLError, TimeoutError, ConnectionError) as err:
            last_error = err
        delay = 15 if isinstance(last_error, urllib.error.HTTPError) and last_error.code == 429 else 2**attempt
        log(f"request to {host_of(url)} failed ({last_error}), retrying in {delay}s ({attempt}/6)")
        time.sleep(delay)
    raise InstallError(f"request to {url} failed: {last_error}")


def get_json(url, headers=None):
    return json.loads(request(url, headers=headers))


def post_json(url, data, headers=None):
    return json.loads(request(url, data=data, headers=headers, method="POST"))


def file_hash(path, algorithm):
    digest = hashlib.new(algorithm)
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url, destination, hashes=None):
    """Downloads `url` to `destination` and verifies it against `hashes` ({algorithm: hex})."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_name(destination.name + ".part")
    last_error = None
    for attempt in range(1, 7):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=120) as response, open(partial, "wb") as out:
                shutil.copyfileobj(response, out, 1024 * 1024)
            for algorithm, expected in (hashes or {}).items():
                if not expected:
                    continue
                actual = file_hash(partial, algorithm)
                if actual.lower() != expected.lower():
                    raise InstallError(f"{algorithm} mismatch for {destination.name}: expected {expected}, got {actual}")
                break
            partial.replace(destination)
            return
        except urllib.error.HTTPError as err:
            last_error = err
            if err.code not in RETRYABLE_STATUS:
                partial.unlink(missing_ok=True)
                raise InstallError(f"download of {destination.name} failed with HTTP {err.code}") from err
        except InstallError as err:
            # a hash mismatch can be a truncated transfer, retry it
            last_error = err
        except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as err:
            last_error = err
        partial.unlink(missing_ok=True)
        delay = 15 if isinstance(last_error, urllib.error.HTTPError) and last_error.code == 429 else 2**attempt
        log(f"download of {destination.name} failed ({last_error}), retrying in {delay}s ({attempt}/6)")
        time.sleep(delay)
    raise InstallError(f"download of {destination.name} failed: {last_error}")


def safe_relative(path):
    """Validates a pack-provided relative path, returning a normalized PurePosixPath or None."""
    if not path or "\\" in path or "\x00" in path:
        return None
    rel = PurePosixPath(path)
    if rel.is_absolute() or any(part in ("", ".", "..") for part in rel.parts):
        return None
    if ":" in rel.parts[0]:
        return None
    return rel


def remove_path(path):
    if path.is_symlink() or path.is_file():
        path.unlink(missing_ok=True)
    elif path.is_dir():
        shutil.rmtree(path, ignore_errors=True)


def minecraft_version_key(version):
    parts = []
    for piece in version.replace("-", ".").split("."):
        digits = "".join(ch for ch in piece if ch.isdigit())
        parts.append(int(digits) if digits else 0)
    return parts


def java_feature_version(minecraft_version):
    """Returns the Java feature release required by a Minecraft release."""
    key = minecraft_version_key(minecraft_version)
    if not key:
        return 21
    if key[0] >= 26:
        return 25
    if key[0] != 1:
        return 21
    minor = key[1] if len(key) > 1 else 0
    patch = key[2] if len(key) > 2 else 0
    if minor > 20 or (minor == 20 and patch >= 5):
        return 21
    if minor >= 17:
        return 17
    return 8


# ---------------------------------------------------------------- pack parsing


class PackFile:
    """A file that has to be downloaded into the server directory."""

    def __init__(self, target, urls, hashes, label):
        self.target = target  # PurePosixPath relative to the server root, or ("@datapacks", name)
        self.urls = urls
        self.hashes = hashes
        self.label = label


def parse_modrinth(archive):
    try:
        index = json.loads(archive.read("modrinth.index.json"))
    except KeyError as err:
        raise InstallError("the modpack archive has no modrinth.index.json") from err
    if index.get("game") != "minecraft":
        raise InstallError(f"unsupported modpack game: {index.get('game')}")

    deps = index.get("dependencies") or {}
    minecraft = str(deps.get("minecraft") or "")
    loader = None
    for key, name in (("neoforge", "neoforge"), ("forge", "forge"), ("fabric-loader", "fabric"), ("quilt-loader", "quilt")):
        if deps.get(key):
            loader = (name, str(deps[key]))
            break

    files = []
    skipped = []
    for entry in index.get("files") or []:
        rel = safe_relative(entry.get("path") or "")
        if rel is None:
            raise InstallError(f"modpack contains an unsafe path: {entry.get('path')!r}")
        if (entry.get("env") or {}).get("server") == "unsupported":
            skipped.append(str(rel))
            continue
        urls = [url for url in entry.get("downloads") or [] if host_allowed(url, MRPACK_HOSTS)]
        if not urls:
            raise InstallError(f"{rel} has no download from an allowed host")
        hashes = entry.get("hashes") or {}
        verified = {"sha512": hashes["sha512"]} if hashes.get("sha512") else {"sha1": hashes.get("sha1", "")}
        files.append(PackFile(rel, urls, verified, str(rel)))

    return {
        "name": index.get("name") or PACK_NAME,
        "version": index.get("versionId") or VERSION_NAME,
        "minecraft": minecraft,
        "loader": loader,
        "files": files,
        "missing": [],
        "client_skipped": skipped,
        "overrides": ["overrides", "server-overrides"],
    }


def curseforge_headers():
    if not CF_API_KEY:
        raise InstallError("the CurseForge API key is not configured")
    return {"x-api-key": CF_API_KEY}


def chunks(items, size):
    for start in range(0, len(items), size):
        yield items[start : start + size]


def modrinth_copies(sha1_hashes):
    """Looks up files on Modrinth by SHA-1 so CurseForge files that block third-party downloads can still be fetched."""
    found = {}
    for batch in chunks(sorted(sha1_hashes), 500):
        try:
            versions = post_json(f"{MODRINTH_API}/version_files", {"hashes": batch, "algorithm": "sha1"})
        except InstallError as err:
            log(f"Modrinth lookup failed: {err}")
            continue
        for sha1, version in versions.items():
            for file in version.get("files") or []:
                if (file.get("hashes") or {}).get("sha1", "").lower() == sha1.lower() and host_allowed(file.get("url", ""), MRPACK_HOSTS):
                    found[sha1.lower()] = file["url"]
    return found


def parse_curseforge(archive):
    try:
        manifest = json.loads(archive.read("manifest.json"))
    except KeyError as err:
        raise InstallError("the modpack archive has no manifest.json") from err

    minecraft_info = manifest.get("minecraft") or {}
    minecraft = str(minecraft_info.get("version") or "")
    loaders = sorted(minecraft_info.get("modLoaders") or [], key=lambda item: not item.get("primary"))
    loader = None
    if loaders:
        loader_id = str(loaders[0].get("id") or "")
        name, _, version = loader_id.partition("-")
        name = name.lower()
        if name in ("forge", "neoforge", "fabric", "quilt") and version:
            loader = (name, version)

    entries = [entry for entry in manifest.get("files") or [] if entry.get("required", True)]
    file_ids = sorted({int(entry["fileID"]) for entry in entries})
    mod_ids = sorted({int(entry["projectID"]) for entry in entries})
    headers = curseforge_headers()

    log(f"resolving {len(file_ids)} CurseForge files")
    cf_files = {}
    for batch in chunks(file_ids, 500):
        for file in post_json(f"{CURSEFORGE_API}/v1/mods/files", {"fileIds": batch}, headers).get("data") or []:
            cf_files[int(file["id"])] = file
    cf_mods = {}
    for batch in chunks(mod_ids, 500):
        for mod in post_json(f"{CURSEFORGE_API}/v1/mods", {"modIds": batch, "filterPcOnly": False}, headers).get("data") or []:
            cf_mods[int(mod["id"])] = mod

    def sha1_of(file):
        for item in file.get("hashes") or []:
            if item.get("algo") == 1:
                return str(item.get("value", "")).lower()
        return ""

    blocked = [f for f in cf_files.values() if not f.get("downloadUrl")]
    fallbacks = modrinth_copies({sha1_of(f) for f in blocked if sha1_of(f)}) if blocked else {}

    files = []
    missing = []
    skipped = []
    for entry in entries:
        file = cf_files.get(int(entry["fileID"]))
        mod = cf_mods.get(int(entry["projectID"])) or {}
        mod_name = mod.get("name") or f"project {entry['projectID']}"
        if file is None:
            raise InstallError(f"CurseForge file {entry['fileID']} of {mod_name} could not be resolved")

        filename = str(file.get("fileName") or "")
        if safe_relative(filename) is None or "/" in filename:
            raise InstallError(f"CurseForge file {entry['fileID']} has an unsafe name: {filename!r}")

        class_id = mod.get("classId") or CF_CLASS_MODS
        game_versions = {str(v).lower() for v in file.get("gameVersions") or []}
        if class_id in (CF_CLASS_RESOURCE_PACKS, CF_CLASS_SHADERS, CF_CLASS_WORLDS):
            skipped.append(filename)
            continue
        if "client" in game_versions and "server" not in game_versions:
            skipped.append(filename)
            continue

        if class_id == CF_CLASS_DATAPACKS:
            target = ("@datapacks", filename)
        else:
            target = PurePosixPath("mods") / filename

        sha1 = sha1_of(file)
        url = file.get("downloadUrl")
        if url and not host_allowed(url, CURSEFORGE_HOSTS):
            raise InstallError(f"{filename} has a download URL on an unexpected host: {host_of(url)}")
        if not url and sha1 in fallbacks:
            log(f"{filename} blocks third-party downloads on CurseForge, using the identical Modrinth copy")
            url = fallbacks[sha1]
        if not url:
            slug = mod.get("slug") or entry["projectID"]
            missing.append(
                {
                    "name": mod_name,
                    "file": filename,
                    "target": str(target[1] if isinstance(target, tuple) else target),
                    "url": f"https://www.curseforge.com/minecraft/mc-mods/{slug}/files/{entry['fileID']}",
                }
            )
            continue
        files.append(PackFile(target, [url], {"sha1": sha1}, filename))

    return {
        "name": manifest.get("name") or PACK_NAME,
        "version": manifest.get("version") or VERSION_NAME,
        "minecraft": minecraft,
        "loader": loader,
        "files": files,
        "missing": missing,
        "client_skipped": skipped,
        "overrides": [str(manifest.get("overrides") or "overrides")],
    }


# ------------------------------------------------------------------- staging


def stage_path(target):
    if isinstance(target, tuple):
        return TMP / "files" / "@datapacks" / target[1]
    return TMP / "files" / target


def stage_files(files):
    total = len(files)
    if total == 0:
        return
    done = 0

    def fetch(item):
        last = None
        for url in item.urls:
            try:
                download(url, stage_path(item.target), item.hashes)
                return item
            except InstallError as err:
                last = err
        raise last

    with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as pool:
        futures = [pool.submit(fetch, item) for item in files]
        try:
            for future in concurrent.futures.as_completed(futures):
                item = future.result()
                done += 1
                log(f"downloaded {done}/{total}: {item.label}")
                progress(10 + done * 65 / total, f"Downloading files ({done}/{total})")
        except BaseException:
            for future in futures:
                future.cancel()
            raise


def level_name():
    properties = ROOT / "server.properties"
    if properties.is_file():
        for line in properties.read_text(encoding="utf-8", errors="replace").splitlines():
            key, sep, value = line.partition("=")
            if sep and key.strip() == "level-name" and value.strip():
                rel = safe_relative(value.strip())
                if rel is not None:
                    return str(rel)
    return "world"


def move_staged():
    staged_root = TMP / "files"
    if not staged_root.exists():
        return
    datapacks = staged_root / "@datapacks"
    if datapacks.exists():
        target_dir = ROOT / level_name() / "datapacks"
        target_dir.mkdir(parents=True, exist_ok=True)
        for item in datapacks.iterdir():
            remove_path(target_dir / item.name)
            shutil.move(str(item), str(target_dir / item.name))
        shutil.rmtree(datapacks)
    for path in sorted(staged_root.rglob("*")):
        if path.is_dir():
            continue
        rel = path.relative_to(staged_root)
        destination = ROOT / rel
        destination.parent.mkdir(parents=True, exist_ok=True)
        remove_path(destination)
        shutil.move(str(path), str(destination))


def apply_overrides(archive, prefixes):
    """Extracts the override folders of the pack. Returns the top-level names it wrote."""
    written = set()
    decided = {}
    for prefix in prefixes:
        base = prefix.strip("/") + "/"
        for info in archive.infolist():
            if not info.filename.startswith(base) or info.is_dir():
                continue
            rel = safe_relative(info.filename[len(base) :])
            if rel is None:
                raise InstallError(f"modpack override has an unsafe path: {info.filename!r}")
            top = rel.parts[0]
            if top.lower() in CLIENT_OVERRIDES:
                continue
            if top in PROTECTED:
                if top not in decided:
                    decided[top] = not (ROOT / top).exists()
                    if not decided[top]:
                        log(f"keeping the existing {top} (not overwritten by the modpack)")
                if not decided[top]:
                    continue
            destination = ROOT / rel
            destination.parent.mkdir(parents=True, exist_ok=True)
            remove_path(destination)
            with archive.open(info) as source, open(destination, "wb") as out:
                shutil.copyfileobj(source, out)
            written.add(top)
    return sorted(written)


def remove_previous_install(previous):
    """Removes the mods, scripts and mod loader of the previous installation (replace mode)."""
    remove_path(ROOT / "mods")
    if previous:
        for name in REPLACEABLE_SCRIPT_DIRS:
            if name in (previous.get("managed") or []):
                remove_path(ROOT / name)
    for name in (
        "libraries",
        ".fabric",
        ".quilt",
        "versions",
        "run.sh",
        "run.bat",
        "unix_args.txt",
        "fabric-server-launch.jar",
        "fabric-server-launcher.properties",
        "quilt-server-launch.jar",
        SERVER_JARFILE,
    ):
        rel = safe_relative(name)
        if rel is not None:
            remove_path(ROOT / rel)
    for pattern in ("forge-*.jar", "neoforge-*.jar", "minecraft_server*.jar"):
        for path in ROOT.glob(pattern):
            remove_path(path)


# ------------------------------------------------------------ loader install


def mcjars_build(kind, minecraft, version):
    try:
        builds = get_json(f"{MCJARS_API}/builds/{kind}/{urllib.parse.quote(minecraft)}").get("builds") or []
    except InstallError as err:
        log(f"mcjars lookup failed: {err}")
        return None, []
    wanted = {version, version.removeprefix(f"{minecraft}-")}
    for build in builds:
        if str(build.get("projectVersionId")) in wanted or str(build.get("name")) in wanted:
            return build, builds
    return None, builds


def run_mcjars_installation(build):
    for step_group in build.get("installation") or []:
        for step in step_group:
            kind = step.get("type")
            if kind == "download":
                rel = safe_relative(str(step.get("file") or ""))
                if rel is None:
                    raise InstallError("mcjars returned an unsafe installation path")
                log(f"downloading {rel}")
                download(step["url"], ROOT / rel)
            elif kind == "unzip":
                rel = safe_relative(str(step.get("file") or ""))
                location = safe_relative(str(step.get("location") or ".").strip("./") or "x")
                if rel is None or location is None:
                    raise InstallError("mcjars returned an unsafe installation path")
                target = ROOT if str(step.get("location") or ".").strip("./") == "" else ROOT / location
                with zipfile.ZipFile(ROOT / rel) as archive:
                    for info in archive.infolist():
                        member = safe_relative(info.filename.rstrip("/"))
                        if member is None:
                            raise InstallError(f"loader archive has an unsafe path: {info.filename!r}")
                        destination = target / member
                        if info.is_dir():
                            destination.mkdir(parents=True, exist_ok=True)
                            continue
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        with archive.open(info) as source, open(destination, "wb") as out:
                            shutil.copyfileobj(source, out)
            elif kind == "remove":
                rel = safe_relative(str(step.get("location") or ""))
                if rel is None:
                    raise InstallError("mcjars returned an unsafe installation path")
                remove_path(ROOT / rel)
            else:
                raise InstallError(f"unsupported mcjars installation step: {kind}")


def ensure_java(minecraft):
    feature = java_feature_version(minecraft)
    java_home = TMP / f"jre-{feature}"
    java = java_home / "bin" / "java"
    if java.exists():
        return java
    arch = {"x86_64": "x64", "amd64": "x64", "aarch64": "aarch64", "arm64": "aarch64"}.get(platform.machine().lower(), "x64")
    url = f"https://api.adoptium.net/v3/binary/latest/{feature}/ga/linux/{arch}/jre/hotspot/normal/eclipse"
    archive_path = TMP / f"jre-{feature}.tar.gz"
    log(f"downloading a Java {feature} runtime to run the loader installer")
    download(url, archive_path)
    extract_to = TMP / f"jre-{feature}-extract"
    with tarfile.open(archive_path) as archive:
        archive.extractall(extract_to, filter="data")
    top = next(extract_to.iterdir())
    top.rename(java_home)
    shutil.rmtree(extract_to, ignore_errors=True)
    archive_path.unlink(missing_ok=True)
    return java


def run_official_installer(loader, version, minecraft):
    if loader == "forge":
        full = version if version.startswith(f"{minecraft}-") else f"{minecraft}-{version}"
        url = f"https://maven.minecraftforge.net/net/minecraftforge/forge/{full}/forge-{full}-installer.jar"
    elif minecraft == "1.20.1":
        full = version if version.startswith("1.20.1-") else f"1.20.1-{version}"
        url = f"https://maven.neoforged.net/releases/net/neoforged/forge/{full}/forge-{full}-installer.jar"
    else:
        url = f"https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"

    java = ensure_java(minecraft)
    installer = TMP / "loader-installer.jar"
    log(f"downloading the official {loader} {version} installer")
    download(url, installer)
    log(f"running the {loader} installer, this can take a few minutes")
    result = subprocess.run(
        [str(java), "-jar", str(installer), "--installServer", str(ROOT)],
        cwd=str(ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    for line in result.stdout.splitlines()[-25:]:
        log(f"  {line}")
    if result.returncode != 0:
        raise InstallError(f"the {loader} installer exited with code {result.returncode}")
    for leftover in ROOT.glob("*installer.jar.log"):
        leftover.unlink(missing_ok=True)

    # Modern Forge/NeoForge start through @unix_args.txt; copy it to the server
    # root like the stock Forge eggs do so `@unix_args.txt` startups work.
    args = sorted(ROOT.glob("libraries/net/*/*/*/unix_args.txt"))
    if args:
        shutil.copyfile(args[-1], ROOT / "unix_args.txt")
        log("installed a modern Forge/NeoForge server; the startup command must use @unix_args.txt")
        return
    jars = [p for p in ROOT.glob("forge-*.jar") if "installer" not in p.name]
    if jars:
        jars.sort(key=lambda p: ("universal" in p.name, p.name))
        target = ROOT / SERVER_JARFILE
        remove_path(target)
        jars[0].rename(target)


def fabric_meta_launcher(minecraft, version):
    installers = get_json("https://meta.fabricmc.net/v2/versions/installer")
    stable = next((item for item in installers if item.get("stable")), installers[0])
    url = (
        "https://meta.fabricmc.net/v2/versions/loader/"
        f"{urllib.parse.quote(minecraft)}/{urllib.parse.quote(version)}/{stable['version']}/server/jar"
    )
    download(url, ROOT / SERVER_JARFILE)


def install_loader(loader, version, minecraft):
    if not minecraft:
        raise InstallError("the modpack does not declare a Minecraft version")
    kind = {"fabric": "FABRIC", "quilt": "QUILT", "forge": "FORGE", "neoforge": "NEOFORGE"}[loader]
    build, builds = mcjars_build(kind, minecraft, version)
    if build:
        log(f"installing {loader} {version} for Minecraft {minecraft}")
        run_mcjars_installation(build)
    elif loader == "fabric":
        log(f"installing fabric {version} for Minecraft {minecraft} from meta.fabricmc.net")
        fabric_meta_launcher(minecraft, version)
    elif loader in ("forge", "neoforge"):
        try:
            run_official_installer(loader, version, minecraft)
        except InstallError as err:
            fallback = next((b for b in builds if not b.get("experimental")), None)
            if not fallback:
                raise
            log(f"{err}; falling back to {loader} {fallback.get('name')}")
            run_mcjars_installation(fallback)
    else:
        fallback = next((b for b in builds if not b.get("experimental")), builds[0] if builds else None)
        if not fallback:
            raise InstallError(f"no {loader} build is available for Minecraft {minecraft}")
        log(f"{loader} {version} is unavailable, installing {fallback.get('name')} instead")
        run_mcjars_installation(fallback)

    produced = ROOT / "server.jar"
    if SERVER_JARFILE != "server.jar" and produced.exists():
        target = ROOT / SERVER_JARFILE
        target.parent.mkdir(parents=True, exist_ok=True)
        remove_path(target)
        produced.rename(target)


# ------------------------------------------------------- client-only cleanup


def read_mod_metadata(jar):
    """Returns (mod ids, client_only, required dependency ids) of a mod jar."""
    ids, required = set(), set()
    client_only = False
    try:
        with zipfile.ZipFile(jar) as archive:
            names = set(archive.namelist())
            if "fabric.mod.json" in names:
                data = json.loads(archive.read("fabric.mod.json").decode("utf-8", errors="replace"), strict=False)
                if data.get("id"):
                    ids.add(str(data["id"]).lower())
                client_only = client_only or data.get("environment") == "client"
                required.update(str(key).lower() for key in (data.get("depends") or {}))
            if "quilt.mod.json" in names:
                data = json.loads(archive.read("quilt.mod.json").decode("utf-8", errors="replace"), strict=False)
                loader = data.get("quilt_loader") or {}
                if loader.get("id"):
                    ids.add(str(loader["id"]).lower())
                client_only = client_only or (data.get("minecraft") or {}).get("environment") == "client"
                for dep in loader.get("depends") or []:
                    dep_id = dep if isinstance(dep, str) else (dep or {}).get("id")
                    if dep_id and not (isinstance(dep, dict) and dep.get("optional")):
                        required.add(str(dep_id).lower())
            for meta in ("META-INF/mods.toml", "META-INF/neoforge.mods.toml"):
                if meta not in names:
                    continue
                try:
                    data = tomllib.loads(archive.read(meta).decode("utf-8", errors="replace"))
                except tomllib.TOMLDecodeError:
                    continue
                for mod in data.get("mods") or []:
                    if mod.get("modId"):
                        ids.add(str(mod["modId"]).lower())
                for dep_list in (data.get("dependencies") or {}).values():
                    for dep in dep_list if isinstance(dep_list, list) else []:
                        mandatory = dep.get("mandatory") is True or str(dep.get("type", "")).lower() == "required"
                        if mandatory and str(dep.get("side", "BOTH")).upper() != "CLIENT" and dep.get("modId"):
                            required.add(str(dep["modId"]).lower())
    except (zipfile.BadZipFile, OSError, ValueError) as err:
        log(f"could not inspect {jar.name}: {err}")
    return ids, client_only, required


def excluded_by_admin(jar, ids):
    name = jar.name.lower()
    for pattern in EXCLUDE_PATTERNS:
        if fnmatch.fnmatch(name, pattern) or pattern in ids:
            return True
    return False


def remove_client_only_mods():
    mods_dir = ROOT / "mods"
    if not mods_dir.is_dir():
        return []
    jars = sorted(mods_dir.glob("*.jar"))
    metadata = {jar: read_mod_metadata(jar) for jar in jars}

    candidates = {}
    for jar, (ids, client_only, _) in metadata.items():
        if excluded_by_admin(jar, ids):
            candidates[jar] = "excluded by the panel administrator"
        elif client_only:
            candidates[jar] = "declared client-only"
        elif ids & KNOWN_CLIENT_ONLY:
            candidates[jar] = "known client-only mod"

    # Keep candidates another remaining mod depends on.
    required = set()
    for jar, (_, _, deps) in metadata.items():
        if jar not in candidates:
            required |= deps
    changed = True
    while changed:
        changed = False
        for jar in list(candidates):
            ids, _, deps = metadata[jar]
            if ids & required and candidates[jar] != "excluded by the panel administrator":
                log(f"keeping {jar.name}: required by another mod")
                del candidates[jar]
                required |= deps
                changed = True

    for jar, reason in candidates.items():
        log(f"removing {jar.name} ({reason})")
        jar.unlink(missing_ok=True)
    return sorted(jar.name for jar in candidates)


# ---------------------------------------------------------------------- main


def read_previous_marker():
    try:
        return json.loads(MARKER.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def write_missing_file(missing):
    if not missing:
        MISSING_FILE.unlink(missing_ok=True)
        return
    lines = [
        "The following modpack files block third-party downloads on CurseForge and could not be",
        "downloaded automatically. Download them manually and upload them to the listed folder.",
        "",
    ]
    for item in missing:
        lines.append(f"- {item['name']}: {item['file']} -> {item['target']}")
        lines.append(f"  {item['url']}")
    MISSING_FILE.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main():
    if SOURCE not in ("modrinth", "curseforge"):
        raise InstallError(f"unknown modpack source {SOURCE!r}")
    allowed = MRPACK_HOSTS if SOURCE == "modrinth" else CURSEFORGE_HOSTS
    if not host_allowed(PACK_URL, allowed):
        raise InstallError(f"modpack download host is not allowed: {host_of(PACK_URL)}")

    log(f"installing {PACK_NAME} {VERSION_NAME} from {SOURCE} ({'clean install' if INSTALL_MODE == 'wipe' else 'replacing the previous modpack'})")
    ROOT.mkdir(parents=True, exist_ok=True)
    previous = read_previous_marker()
    shutil.rmtree(TMP, ignore_errors=True)
    TMP.mkdir(parents=True)

    progress(2, "Downloading modpack")
    pack_path = TMP / "pack.zip"
    download(PACK_URL, pack_path, {"sha1": PACK_SHA1} if PACK_SHA1 else None)

    with zipfile.ZipFile(pack_path) as archive:
        progress(6, "Reading modpack")
        pack = parse_modrinth(archive) if SOURCE == "modrinth" else parse_curseforge(archive)
        loader = pack["loader"]
        log(
            f"pack targets Minecraft {pack['minecraft'] or 'unknown'}"
            + (f" with {loader[0]} {loader[1]}" if loader else " without a mod loader")
        )
        for name in pack["client_skipped"]:
            log(f"skipping client-only file {name}")

        progress(10, "Downloading files")
        stage_files(pack["files"])

        progress(76, "Replacing previous installation")
        remove_previous_install(previous)
        move_staged()

        progress(80, "Applying overrides")
        managed = apply_overrides(archive, pack["overrides"])

    if loader:
        progress(84, f"Installing {loader[0]}")
        install_loader(loader[0], loader[1], pack["minecraft"])

    progress(95, "Removing client-only mods")
    removed = remove_client_only_mods()

    if ACCEPT_EULA:
        (ROOT / "eula.txt").write_text("eula=true\n", encoding="utf-8")

    write_missing_file(pack["missing"])
    marker = {
        "source": SOURCE,
        "project_id": PROJECT_ID,
        "version_id": VERSION_ID,
        "name": PACK_NAME,
        "version_name": VERSION_NAME or pack["version"],
        "icon_url": ICON_URL or None,
        "minecraft_version": pack["minecraft"] or None,
        "loader": loader[0] if loader else None,
        "loader_version": loader[1] if loader else None,
        "installed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "managed": sorted(set(managed) | {"mods"}),
        "removed_client_mods": removed,
        "missing_files": [item["file"] for item in pack["missing"]],
    }
    MARKER.write_text(json.dumps(marker, indent=2) + "\n", encoding="utf-8")

    shutil.rmtree(TMP, ignore_errors=True)
    progress(100, "Done")
    if pack["missing"]:
        log(f"WARNING: {len(pack['missing'])} file(s) must be downloaded manually, see {MISSING_FILE.name}")
    log(f"installed {PACK_NAME} {marker['version_name']} ({len(pack['files'])} files)")


if __name__ == "__main__":
    try:
        main()
    except InstallError as err:
        log(f"installation failed: {err}")
        report_failure(str(err))
        shutil.rmtree(TMP, ignore_errors=True)
        sys.exit(1)
    except Exception as err:  # noqa: BLE001 - surface anything unexpected in the console
        log(f"installation failed unexpectedly: {err!r}")
        report_failure(f"unexpected error: {err}")
        shutil.rmtree(TMP, ignore_errors=True)
        sys.exit(1)
