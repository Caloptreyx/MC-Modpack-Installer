import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';
import {
  type AdminSettings,
  adminSettingsSchema,
  type UpdateAdminSettings,
  updateAdminSettingsSchema,
} from '../../lib/schemas.ts';
import { adminModpacksBase } from '../paths.ts';

export default async (settings: UpdateAdminSettings): Promise<AdminSettings> => {
  const { data } = await axiosInstance.put(
    `${adminModpacksBase}/settings`,
    serializeForApi(updateAdminSettingsSchema, settings),
  );
  return parseFromApi(adminSettingsSchema, data.settings);
};
