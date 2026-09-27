import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, parsePaginationFromApi } from '@/lib/serialization/api-transform.ts';
import {
  type Loader,
  loaderSchema,
  type ModpackVersion,
  modpackVersionSchema,
  type Provider,
  type ReleaseType,
} from '../../lib/schemas.ts';
import { projectBase } from '../paths.ts';

export interface GetVersionsParams {
  page: number;
  perPage: number;
  search?: string;
  loader?: Loader | null;
  gameVersion?: string | null;
  releaseType?: ReleaseType | null;
}

export interface VersionsResult {
  versions: Pagination<ModpackVersion>;
  /** Loaders available with the other filters applied. */
  loaders: Loader[];
  /** Minecraft versions available with the other filters applied, newest first. */
  gameVersions: string[];
}

const facetsSchema = z.object({
  loaders: z.array(loaderSchema),
  gameVersions: z.array(z.string()),
});

export default async (
  serverUuid: string,
  provider: Provider,
  projectId: string,
  params: GetVersionsParams,
): Promise<VersionsResult> => {
  const { data } = await axiosInstance.get(`${projectBase(serverUuid, provider, projectId)}/versions`, {
    params: {
      page: params.page,
      per_page: params.perPage,
      search: params.search || undefined,
      loader: params.loader || undefined,
      game_version: params.gameVersion || undefined,
      release_type: params.releaseType || undefined,
    },
  });
  const facets = parseFromApi(facetsSchema, { loaders: data.loaders, game_versions: data.game_versions });

  return {
    versions: parsePaginationFromApi(modpackVersionSchema, data.versions),
    loaders: facets.loaders,
    gameVersions: facets.gameVersions,
  };
};
