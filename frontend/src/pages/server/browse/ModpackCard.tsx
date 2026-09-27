import { faClock, faDownload, faHeart, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, Stack } from '@mantine/core';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import LoaderBadge from '../../../components/LoaderBadge.tsx';
import ModpackIcon from '../../../components/ModpackIcon.tsx';
import { compactNumber, gameVersionRange } from '../../../lib/format.ts';
import type { ModpackSummary } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

interface Props {
  modpack: ModpackSummary;
  installed: boolean;
  onOpen: () => void;
  onInstall: () => void;
}

export default function ModpackCard({ modpack, installed, onOpen, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const range = gameVersionRange(modpack.gameVersions);

  return (
    <Card hoverable onClick={onOpen} p='md' h='100%'>
      <Stack gap='sm' h='100%' justify='space-between'>
        <Stack gap='sm'>
          <Group wrap='nowrap' align='flex-start' gap='md'>
            <ModpackIcon url={modpack.iconUrl} name={modpack.name} />
            <Stack gap={2} style={{ minWidth: 0, flex: 1 }}>
              <Group gap={6} wrap='nowrap'>
                <Text fw={600} size='lg' truncate>
                  {modpack.name}
                </Text>
                {installed && (
                  <Badge color='green' variant='filled' size='xs'>
                    {tExt('pages.server.modpacks.versions.installed', {})}
                  </Badge>
                )}
              </Group>
              {modpack.author && (
                <Text size='xs' c='dimmed' truncate>
                  {tExt('pages.server.modpacks.card.by', { author: modpack.author })}
                </Text>
              )}
              <Text size='sm' lineClamp={2} mt={4}>
                {modpack.summary}
              </Text>
            </Stack>
          </Group>

          <Group gap={6}>
            {modpack.loaders.map((loader) => (
              <LoaderBadge key={loader} loader={loader} />
            ))}
            {range && (
              <Badge color='gray' variant='light' size='sm'>
                {range}
              </Badge>
            )}
            {modpack.categories.slice(0, 3).map((category) => (
              <Badge key={category} color='gray' variant='outline' size='sm'>
                {category}
              </Badge>
            ))}
            {modpack.clientOnly && (
              <Tooltip label={tExt('pages.server.modpacks.card.clientOnlyHint', {})}>
                <Badge
                  color='red'
                  variant='light'
                  size='sm'
                  leftSection={<FontAwesomeIcon icon={faTriangleExclamation} />}
                >
                  {tExt('pages.server.modpacks.card.clientOnly', {})}
                </Badge>
              </Tooltip>
            )}
          </Group>
        </Stack>

        <Group justify='space-between' wrap='nowrap'>
          <Group gap='md' wrap='nowrap'>
            <Tooltip
              label={tExt('pages.server.modpacks.card.downloads', { count: modpack.downloads.toLocaleString() })}
            >
              <Text size='xs' c='dimmed'>
                <FontAwesomeIcon icon={faDownload} /> {compactNumber(modpack.downloads)}
              </Text>
            </Tooltip>
            <Tooltip label={tExt('pages.server.modpacks.card.follows', { count: modpack.follows.toLocaleString() })}>
              <Text size='xs' c='dimmed'>
                <FontAwesomeIcon icon={faHeart} /> {compactNumber(modpack.follows)}
              </Text>
            </Tooltip>
            {modpack.updated && (
              <Text size='xs' c='dimmed' component='span'>
                <FontAwesomeIcon icon={faClock} /> <FormattedTimestamp timestamp={modpack.updated} />
              </Text>
            )}
          </Group>
          <ServerCan action='modpacks.install'>
            <Button
              size='xs'
              variant='light'
              leftSection={<FontAwesomeIcon icon={faDownload} />}
              onClick={(event) => {
                event.stopPropagation();
                onInstall();
              }}
            >
              {tExt('pages.server.modpacks.card.install', {})}
            </Button>
          </ServerCan>
        </Group>
      </Stack>
    </Card>
  );
}
