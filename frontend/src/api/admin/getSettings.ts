import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type AdminSettings, adminSettingsSchema } from '../../lib/schemas.ts';
import { adminModpacksBase } from '../paths.ts';

export default async (): Promise<AdminSettings> => {
  const { data } = await axiosInstance.get(`${adminModpacksBase}/settings`);
  return parseFromApi(adminSettingsSchema, data.settings);
};
