import { defineEnglishItem, defineTranslations } from 'shared';

const translations = defineTranslations({
  items: {
    modpack: defineEnglishItem('Modpack', 'Modpacks'),
    version: defineEnglishItem('Version', 'Versions'),
    image: defineEnglishItem('Image', 'Images'),
  },
  translations: {
    providers: {
      modrinth: 'Modrinth',
      curseforge: 'CurseForge',
    },
    loaders: {
      fabric: 'Fabric',
      forge: 'Forge',
      neoforge: 'NeoForge',
      quilt: 'Quilt',
      unknown: 'Unknown',
    },
    releaseTypes: {
      release: 'Release',
      beta: 'Beta',
      alpha: 'Alpha',
    },
    pages: {
      server: {
        modpacks: {
          title: 'Modpacks',
          detected: 'Detected {source}',
          detectedSource: {
            modpack: 'from the installed modpack',
            files: 'from the server files',
            egg: 'from the server egg',
          },
          notDetected: 'No mod loader detected on this server',
          blocked: {
            title: 'No modpack platforms available',
            content: 'Modrinth and CurseForge are disabled. Ask an administrator to enable one of them.',
          },
          browse: {
            searchPlaceholder: 'Search modpacks…',
            sort: 'Sort by',
            sorts: {
              relevance: 'Relevance',
              downloads: 'Downloads',
              follows: 'Follows',
              newest: 'Newest',
              updated: 'Recently updated',
            },
            loader: 'Mod loader',
            gameVersion: 'Minecraft version',
            category: 'Category',
            anyLoader: 'Any loader',
            anyVersion: 'Any version',
            anyCategory: 'Any category',
            hideClientOnly: 'Hide client-only packs',
            matchServer: 'Match this server',
            matchServerHint: 'Only show packs for {loader} on Minecraft {version}',
            clearFilters: 'Clear filters',
            results: '{count} modpacks',
            curseforgeUnavailable: 'CurseForge needs an API key configured by an administrator.',
            empty: {
              title: 'No modpacks found',
              description: 'Try a different search term or remove some filters.',
            },
          },
          card: {
            by: 'by {author}',
            downloads: '{count} downloads',
            follows: '{count} follows',
            updated: 'Updated',
            clientOnly: 'Client only',
            clientOnlyHint: 'The author marked this modpack as not supported on servers.',
            install: 'Install',
            view: 'View details',
          },
          installed: {
            title: 'Installed modpack',
            version: 'Version {version}',
            installedAt: 'Installed',
            updateAvailable: 'Update available: {version}',
            upToDate: 'Up to date',
            checking: 'Checking for updates…',
            update: 'Update',
            reinstall: 'Reinstall',
            view: 'View',
            missingFiles:
              '{count} file(s) could not be downloaded automatically. See **MODPACK-MISSING-FILES.txt** in the server files.',
            removedMods: '{count} client-only mod(s) were removed from the server.',
          },
          details: {
            back: 'Back to modpacks',
            by: 'by {authors}',
            install: 'Install modpack',
            openOn: 'Open on {provider}',
            stats: {
              downloads: 'Downloads',
              follows: 'Follows',
              created: 'Published',
              updated: 'Updated',
              license: 'License',
            },
            links: {
              website: 'Website',
              source: 'Source',
              issues: 'Issues',
              wiki: 'Wiki',
              discord: 'Discord',
              donation: 'Donate',
            },
            tabs: {
              description: 'Description',
              versions: 'Versions',
              gallery: 'Gallery',
            },
            clientOnly:
              'The author marked this modpack as **not supported on servers**. It will most likely not start as a dedicated server.',
            installedHere: 'This modpack is installed on the server (version {version}).',
            emptyDescription: 'This modpack has no description.',
            unknownProvider: 'Unknown modpack platform.',
          },
          versions: {
            searchPlaceholder: 'Search versions…',
            allLoaders: 'All loaders',
            allGameVersions: 'All Minecraft versions',
            allReleaseTypes: 'All types',
            compatible: 'Compatible with this server',
            columns: {
              version: 'Version',
              type: 'Type',
              gameVersions: 'Minecraft',
              loaders: 'Loaders',
              published: 'Published',
              downloads: 'Downloads',
              size: 'Size',
            },
            installed: 'Installed',
            notDownloadable: 'The author does not allow third-party downloads of this version.',
            moreGameVersions: '+{count} more',
            changelog: 'Changelog',
            install: 'Install',
            empty: 'No versions match the selected filters.',
            changelogModal: {
              title: 'Changelog · {version}',
              empty: 'This version has no changelog.',
            },
          },
          gallery: {
            empty: 'This modpack has no gallery images.',
          },
          install: {
            title: 'Install {name}',
            loader: 'Mod loader',
            gameVersion: 'Minecraft version',
            version: 'Modpack version',
            detectedOption: '{value} (detected)',
            noVersions: 'No installable version matches this loader and Minecraft version.',
            versionOption: '{name} · {type}',
            mode: 'Install mode',
            modes: {
              replace: 'Keep worlds & settings',
              replaceDescription:
                'Replaces the mods, mod loader and modpack configs. Worlds, server.properties, whitelists and bans are kept.',
              wipe: 'Clean install',
              wipeDescription: 'Deletes **all** server files, including worlds, before installing.',
              wipeDisabled: 'Clean installs have been disabled by an administrator.',
            },
            dockerImage: 'Java version (docker image)',
            dockerImageDescription: 'Minecraft {version} needs Java {java}.',
            dockerImageCurrent: '{name} (current)',
            dockerImageRecommended: '{name} (recommended)',
            dockerImageMismatch:
              'The current docker image provides Java {current}, but Minecraft {version} needs Java {java}. The server may not start.',
            acceptEula: 'I accept the [Minecraft EULA](https://aka.ms/MinecraftEULA)',
            warnings: {
              loaderChange: 'The server currently runs **{current}**. Installing replaces it with **{next}**.',
              versionChange: 'The server changes from Minecraft **{current}** to **{next}**.',
              clientOnly: 'This modpack is marked as client-only and will most likely not run on a server.',
              notDownloadable: 'The author does not allow third-party downloads of this version.',
            },
            notice:
              'The server is stopped and reinstalled with this modpack. Installation progress is shown in the console.',
            submit: 'Install',
            toast: 'Installing {name}. Follow the progress in the console.',
          },
        },
      },
      admin: {
        modpacks: {
          tabs: {
            providers: 'Platforms',
            installation: 'Installation',
          },
          providers: {
            modrinth: 'Enable Modrinth',
            modrinthDescription: 'Browse and install modpacks from modrinth.com. No API key needed.',
            curseforge: 'Enable CurseForge',
            curseforgeDescription: 'Browse and install modpacks from curseforge.com. Requires an API key.',
            apiKey: 'CurseForge API key',
            apiKeyDescription:
              'Create a key at [console.curseforge.com](https://console.curseforge.com/). It is stored encrypted and never sent to users.',
            apiKeyConfigured: 'A key is configured. Enter a new key to replace it.',
            apiKeyPlaceholder: 'Paste your CurseForge API key',
            verify: 'Test key',
            remove: 'Remove key',
            verified: 'CurseForge accepted the API key.',
            status: {
              configured: 'Configured',
              missing: 'Not configured',
            },
          },
          installation: {
            allowCleanInstall: 'Allow clean installs',
            allowCleanInstallDescription:
              'Let users delete all server files (including worlds) when installing a modpack.',
            installerImage: 'Installer image',
            installerImageDescription:
              'Docker image the installation runs in. It needs Python 3.12 or newer with glibc (e.g. python:3.13-slim).',
            excludedMods: 'Additional client-only mods',
            excludedModsDescription:
              'Removed from the server after installing. Enter mod ids (e.g. oculus) or filename patterns (e.g. *-client-*.jar).',
          },
          button: {
            save: 'Save',
          },
          toast: {
            saved: 'Settings saved.',
          },
        },
      },
    },
  },
});

export const useExtTranslations = translations.useTranslations.bind(translations);
export const getExtTranslations = translations.getTranslations.bind(translations);

export default translations;
