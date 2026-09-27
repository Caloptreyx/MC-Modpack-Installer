export const serverModpacksBase = (serverUuid: string) => `/api/client/servers/${serverUuid}/modpacks`;

export const projectBase = (serverUuid: string, provider: string, projectId: string) =>
  `${serverModpacksBase(serverUuid)}/projects/${provider}/${encodeURIComponent(projectId)}`;

export const adminModpacksBase = '/api/admin/extensions/dev.caloptreyx.modpacks';
