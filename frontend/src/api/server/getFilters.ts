import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type Filters, filtersSchema, type Provider } from '../../lib/schemas.ts';
import { serverModpacksBase } from '../paths.ts';

export default async (serverUuid: string, provider: Provider): Promise<Filters> => {
  const { data } = await axiosInstance.get(`${serverModpacksBase(serverUuid)}/filters`, { params: { provider } });
  return parseFromApi(filtersSchema, data.filters);
};
