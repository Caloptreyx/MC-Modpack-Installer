import { faCube, faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group } from '@mantine/core';
import Badge from '@/elements/data-display/Badge.tsx';
import ConditionalTooltip from '@/elements/overlays/ConditionalTooltip.tsx';
import LoaderBadge from '../../components/LoaderBadge.tsx';
import type { Overview } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';

/** Badges with the detected mod loader and Minecraft version of the server. */
export default function DetectedSetup({ detected }: { detected: Overview['detected'] }) {
  const { t: tExt } = useExtTranslations();

  if (!detected.loader && !detected.minecraftVersion) {
    return (
      <Badge color='gray' variant='light' leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}>
        {tExt('pages.server.modpacks.notDetected', {})}
      </Badge>
    );
  }

  return (
    <ConditionalTooltip
      enabled={!!detected.source}
      label={
        detected.source
          ? tExt('pages.server.modpacks.detected', {
              source: tExt(`pages.server.modpacks.detectedSource.${detected.source}`, {}),
            })
          : null
      }
    >
      <Group gap={6} wrap='nowrap'>
        {detected.loader && <LoaderBadge loader={detected.loader} size='md' />}
        {detected.minecraftVersion && (
          <Badge color='gray' variant='light' size='md' leftSection={<FontAwesomeIcon icon={faCube} />}>
            {detected.minecraftVersion}
          </Badge>
        )}
      </Group>
    </ConditionalTooltip>
  );
}
