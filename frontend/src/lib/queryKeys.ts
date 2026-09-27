const root = ['extensions', 'dev.caloptreyx.modpacks'] as const;

export const modpackQueryKeys = {
  overview: (serverUuid: string) => [...root, 'server', serverUuid, 'overview'] as const,
  search: (serverUuid: string) => [...root, 'server', serverUuid, 'search'] as const,
  filters: (serverUuid: string, provider: string) => [...root, 'server', serverUuid, 'filters', provider] as const,
  project: (serverUuid: string, provider: string, projectId: string) =>
    [...root, 'server', serverUuid, 'project', provider, projectId] as const,
  versions: (serverUuid: string, provider: string, projectId: string) =>
    [...root, 'server', serverUuid, 'project', provider, projectId, 'versions'] as const,
  version: (serverUuid: string, provider: string, projectId: string, versionId: string) =>
    [...root, 'server', serverUuid, 'project', provider, projectId, 'version', versionId] as const,
  adminSettings: () => [...root, 'admin', 'settings'] as const,
};
