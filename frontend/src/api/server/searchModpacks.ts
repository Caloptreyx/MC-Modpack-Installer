import { axiosInstance } from '@/api/axios.ts';
import { parsePaginationFromApi } from '@/lib/serialization/api-transform.ts';
import {
  type Loader,
  type ModpackSummary,
  modpackSummarySchema,
  type Provider,
  type SortMode,
} from '../../lib/schemas.ts';
import { serverModpacksBase } from '../paths.ts';

export interface SearchModpacksParams {
  provider: Provider;
  page: number;
  perPage: number;
  search?: string;
  loader?: Loader | null;
  gameVersion?: string | null;
  category?: string | null;
  sort: SortMode;
  hideClientOnly: boolean;
}

export default async (serverUuid: string, params: SearchModpacksParams): Promise<Pagination<ModpackSummary>> => {
  const { data } = await axiosInstance.get(`${serverModpacksBase(serverUuid)}/search`, {
    params: {
      provider: params.provider,
      page: params.page,
      per_page: params.perPage,
      search: params.search || undefined,
      loader: params.loader || undefined,
      game_version: params.gameVersion || undefined,
      category: params.category || undefined,
      sort: params.sort,
      hide_client_only: params.hideClientOnly,
    },
  });
  return parsePaginationFromApi(modpackSummarySchema, data.modpacks);
};
