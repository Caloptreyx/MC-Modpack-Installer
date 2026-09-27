import { z } from 'zod';

export const providerSchema = z.enum(['modrinth', 'curseforge']);
export const loaderSchema = z.enum(['fabric', 'forge', 'neoforge', 'quilt']);
export const releaseTypeSchema = z.enum(['release', 'beta', 'alpha']);
export const sortModeSchema = z.enum(['relevance', 'downloads', 'follows', 'newest', 'updated']);
export const textFormatSchema = z.enum(['markdown', 'html']);
export const installModeSchema = z.enum(['replace', 'wipe']);

export type Provider = z.infer<typeof providerSchema>;
export type Loader = z.infer<typeof loaderSchema>;
export type ReleaseType = z.infer<typeof releaseTypeSchema>;
export type SortMode = z.infer<typeof sortModeSchema>;
export type InstallMode = z.infer<typeof installModeSchema>;
export type TextFormat = z.infer<typeof textFormatSchema>;

export const modpackSummarySchema = z.object({
  provider: providerSchema,
  id: z.string(),
  slug: z.string(),
  name: z.string(),
  summary: z.string(),
  author: z.string(),
  iconUrl: z.string().nullable(),
  downloads: z.number(),
  follows: z.number(),
  categories: z.array(z.string()),
  loaders: z.array(loaderSchema),
  gameVersions: z.array(z.string()),
  updated: z.coerce.date().nullable(),
  clientOnly: z.boolean(),
  url: z.string(),
});

export type ModpackSummary = z.infer<typeof modpackSummarySchema>;

export const modpackDetailsSchema = z.object({
  provider: providerSchema,
  id: z.string(),
  slug: z.string(),
  name: z.string(),
  summary: z.string(),
  authors: z.array(z.string()),
  iconUrl: z.string().nullable(),
  downloads: z.number(),
  follows: z.number(),
  categories: z.array(z.string()),
  loaders: z.array(loaderSchema),
  gameVersions: z.array(z.string()),
  created: z.coerce.date().nullable(),
  updated: z.coerce.date().nullable(),
  clientOnly: z.boolean(),
  license: z.string().nullable(),
  url: z.string(),
  body: z.string(),
  bodyFormat: textFormatSchema,
  gallery: z.array(
    z.object({
      url: z.string(),
      title: z.string().nullable(),
      description: z.string().nullable(),
    }),
  ),
  links: z.array(
    z.object({
      kind: z.string(),
      url: z.string(),
    }),
  ),
});

export type ModpackDetails = z.infer<typeof modpackDetailsSchema>;

export const modpackVersionSchema = z.object({
  id: z.string(),
  name: z.string(),
  versionNumber: z.string(),
  releaseType: releaseTypeSchema,
  gameVersions: z.array(z.string()),
  loaders: z.array(loaderSchema),
  published: z.coerce.date(),
  downloads: z.number(),
  fileName: z.string(),
  fileSize: z.number(),
  downloadable: z.boolean(),
});

export type ModpackVersion = z.infer<typeof modpackVersionSchema>;

export const versionDetailsSchema = z.object({
  version: modpackVersionSchema,
  changelog: z.string(),
  changelogFormat: textFormatSchema,
});

export type VersionDetails = z.infer<typeof versionDetailsSchema>;

export const filtersSchema = z.object({
  categories: z.array(z.object({ id: z.string(), name: z.string() })),
  gameVersions: z.array(z.string()),
  loaders: z.array(loaderSchema),
});

export type Filters = z.infer<typeof filtersSchema>;

export const installedModpackSchema = z.object({
  source: providerSchema,
  projectId: z.string(),
  versionId: z.string(),
  name: z.string(),
  versionName: z.string(),
  iconUrl: z.string().nullable(),
  minecraftVersion: z.string().nullable(),
  loader: loaderSchema.nullable(),
  loaderVersion: z.string().nullable(),
  installedAt: z.coerce.date().nullable(),
  removedClientMods: z.array(z.string()),
  missingFiles: z.array(z.string()),
});

export type InstalledModpack = z.infer<typeof installedModpackSchema>;

export const dockerImageSchema = z.object({
  name: z.string(),
  image: z.string(),
  javaVersion: z.number().nullable(),
});

export type DockerImage = z.infer<typeof dockerImageSchema>;

export const overviewSchema = z.object({
  providers: z.object({
    modrinth: z.boolean(),
    curseforge: z.boolean(),
  }),
  allowCleanInstall: z.boolean(),
  detected: z.object({
    loader: loaderSchema.nullable(),
    minecraftVersion: z.string().nullable(),
    source: z.enum(['modpack', 'files', 'egg']).nullable(),
  }),
  installed: installedModpackSchema.nullable(),
  dockerImages: z.array(dockerImageSchema),
  currentImage: z.string(),
  canChangeImage: z.boolean(),
});

export type Overview = z.infer<typeof overviewSchema>;

export const adminSettingsSchema = z.object({
  modrinthEnabled: z.boolean(),
  curseforgeEnabled: z.boolean(),
  curseforgeConfigured: z.boolean(),
  allowCleanInstall: z.boolean(),
  installerImage: z.string(),
  excludedMods: z.array(z.string()),
});

export type AdminSettings = z.infer<typeof adminSettingsSchema>;

export const updateAdminSettingsSchema = z.object({
  modrinthEnabled: z.boolean(),
  curseforgeEnabled: z.boolean(),
  curseforgeApiKey: z.string().max(255).optional(),
  allowCleanInstall: z.boolean(),
  installerImage: z.string().min(1).max(255),
  excludedMods: z.array(z.string().min(1).max(128)).max(200),
});

export type UpdateAdminSettings = z.infer<typeof updateAdminSettingsSchema>;

export const installModpackSchema = z.object({
  provider: providerSchema,
  projectId: z.string(),
  versionId: z.string(),
  mode: installModeSchema,
  acceptEula: z.boolean(),
  dockerImage: z.string().optional(),
});

export type InstallModpack = z.infer<typeof installModpackSchema>;
