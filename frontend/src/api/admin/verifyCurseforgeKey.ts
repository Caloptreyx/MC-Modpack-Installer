import { axiosInstance } from '@/api/axios.ts';
import { adminModpacksBase } from '../paths.ts';

/** Verifies `apiKey`, or the stored key when it is omitted. */
export default async (apiKey?: string): Promise<void> => {
  await axiosInstance.post(`${adminModpacksBase}/curseforge/verify`, { api_key: apiKey || undefined });
};
