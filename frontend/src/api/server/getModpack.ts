import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type ModpackDetails, modpackDetailsSchema, type Provider } from '../../lib/schemas.ts';
import { projectBase } from '../paths.ts';

export default async (serverUuid: string, provider: Provider, projectId: string): Promise<ModpackDetails> => {
  const { data } = await axiosInstance.get(projectBase(serverUuid, provider, projectId));
  return parseFromApi(modpackDetailsSchema, data.modpack);
};
