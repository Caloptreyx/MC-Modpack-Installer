import { axiosInstance } from '@/api/axios.ts';
import { serializeForApi } from '@/lib/serialization/api-transform.ts';
import { type InstallModpack, installModpackSchema } from '../../lib/schemas.ts';
import { serverModpacksBase } from '../paths.ts';

export default async (serverUuid: string, data: InstallModpack): Promise<void> => {
  await axiosInstance.post(`${serverModpacksBase(serverUuid)}/install`, serializeForApi(installModpackSchema, data));
};
