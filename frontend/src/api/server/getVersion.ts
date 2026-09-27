import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type Provider, type VersionDetails, versionDetailsSchema } from '../../lib/schemas.ts';
import { projectBase } from '../paths.ts';

export default async (
  serverUuid: string,
  provider: Provider,
  projectId: string,
  versionId: string,
): Promise<VersionDetails> => {
  const { data } = await axiosInstance.get(
    `${projectBase(serverUuid, provider, projectId)}/versions/${encodeURIComponent(versionId)}`,
  );
  return parseFromApi(versionDetailsSchema, data.version);
};
