import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type Overview, overviewSchema } from '../../lib/schemas.ts';
import { serverModpacksBase } from '../paths.ts';

export default async (serverUuid: string): Promise<Overview> => {
  const { data } = await axiosInstance.get(serverModpacksBase(serverUuid));
  return parseFromApi(overviewSchema, data);
};
