import { Group } from '@mantine/core';
import type { Provider } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

export const providerColors: Record<Provider, string> = {
  modrinth: '#1bd96a',
  curseforge: '#f16436',
};

/** A dot in the platform's brand color. */
export function ProviderDot({ provider }: { provider: Provider }) {
  return (
    <span
      style={{
        display: 'inline-block',
        width: 8,
        height: 8,
        borderRadius: '50%',
        backgroundColor: providerColors[provider],
      }}
    />
  );
}

/** The platform name with its brand color as a dot. */
export default function ProviderMark({ provider }: { provider: Provider }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Group gap={6} wrap='nowrap' component='span'>
      <ProviderDot provider={provider} />
      {tExt(`providers.${provider}`, {})}
    </Group>
  );
}
