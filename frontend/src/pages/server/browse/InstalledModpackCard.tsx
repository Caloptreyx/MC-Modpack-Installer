import { faArrowsRotate, faArrowUpRightFromSquare, faCircleCheck, faCircleUp } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, Stack } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import getVersions from '../../../api/server/getVersions.ts';
import LoaderBadge from '../../../components/LoaderBadge.tsx';
import ModpackIcon from '../../../components/ModpackIcon.tsx';
import ProviderMark from '../../../components/ProviderMark.tsx';
import { modpackQueryKeys } from '../../../lib/queryKeys.ts';
import type { InstalledModpack } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';
import type { InstallPreset } from '../modals/InstallModpackModal.tsx';

interface Props {
  installed: InstalledModpack;
  providerAvailable: boolean;
  onView: () => void;
  onInstall: (preset: InstallPreset) => void;
}

export default function InstalledModpackCard({ installed, providerAvailable, onView, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();

  // Newest version for the installed loader and Minecraft version.
  const latest = useQuery({
    queryKey: [
      ...modpackQueryKeys.versions(server.uuid, installed.source, installed.projectId),
      'latest',
      installed.loader,
      installed.minecraftVersion,
    ],
    queryFn: () =>
      getVersions(server.uuid, installed.source, installed.projectId, {
        page: 1,
        perPage: 1,
        loader: installed.loader,
        gameVersion: installed.minecraftVersion,
      }),
    enabled: providerAvailable,
  });

  const newest = latest.data?.versions.data[0] ?? null;
  const updateAvailable = !!newest && newest.id !== installed.versionId && newest.downloadable;

  return (
    <Card p='md' mb='md' leftStripeClassName={updateAvailable ? 'bg-blue-500' : 'bg-green-600'}>
      <Stack gap='sm'>
        <Group justify='space-between' wrap='wrap' gap='md'>
          <Group wrap='nowrap' gap='md' style={{ minWidth: 0 }}>
            <ModpackIcon url={installed.iconUrl} name={installed.name} size={56} />
            <Stack gap={2} style={{ minWidth: 0 }}>
              <Text size='xs' c='dimmed' tt='uppercase' fw={600}>
                {tExt('pages.server.modpacks.installed.title', {})}
              </Text>
              <Text fw={600} size='lg' truncate>
                {installed.name}
              </Text>
              <Group gap={6}>
                <Text size='sm' c='dimmed'>
                  {tExt('pages.server.modpacks.installed.version', { version: installed.versionName })}
                </Text>
                {installed.loader && <LoaderBadge loader={installed.loader} />}
                {installed.minecraftVersion && (
                  <Badge color='gray' variant='light' size='sm'>
                    {installed.minecraftVersion}
                  </Badge>
                )}
                <Text size='xs' c='dimmed' component='div'>
                  <ProviderMark provider={installed.source} />
                </Text>
                {installed.installedAt && (
                  <Text size='xs' c='dimmed' component='span'>
                    {tExt('pages.server.modpacks.installed.installedAt', {})}{' '}
                    <FormattedTimestamp timestamp={installed.installedAt} />
                  </Text>
                )}
              </Group>
            </Stack>
          </Group>

          <Group gap='sm'>
            {providerAvailable &&
              (latest.isLoading ? (
                <Text size='sm' c='dimmed'>
                  {tExt('pages.server.modpacks.installed.checking', {})}
                </Text>
              ) : updateAvailable && newest ? (
                <Badge color='blue' variant='light' size='lg' leftSection={<FontAwesomeIcon icon={faCircleUp} />}>
                  {tExt('pages.server.modpacks.installed.updateAvailable', { version: newest.name })}
                </Badge>
              ) : newest ? (
                <Badge color='green' variant='light' size='lg' leftSection={<FontAwesomeIcon icon={faCircleCheck} />}>
                  {tExt('pages.server.modpacks.installed.upToDate', {})}
                </Badge>
              ) : null)}
            {providerAvailable && (
              <Button
                variant='default'
                leftSection={<FontAwesomeIcon icon={faArrowUpRightFromSquare} />}
                onClick={onView}
              >
                {tExt('pages.server.modpacks.installed.view', {})}
              </Button>
            )}
            {providerAvailable && (
              <ServerCan action='modpacks.install'>
                <Button
                  color='blue'
                  leftSection={<FontAwesomeIcon icon={updateAvailable ? faCircleUp : faArrowsRotate} />}
                  onClick={() =>
                    onInstall({
                      loader: installed.loader,
                      gameVersion: installed.minecraftVersion,
                      versionId: updateAvailable && newest ? newest.id : installed.versionId,
                    })
                  }
                >
                  {updateAvailable
                    ? tExt('pages.server.modpacks.installed.update', {})
                    : tExt('pages.server.modpacks.installed.reinstall', {})}
                </Button>
              </ServerCan>
            )}
          </Group>
        </Group>

        {installed.missingFiles.length > 0 && (
          <Alert color='yellow'>
            <Text size='sm' component='div'>
              {tExt('pages.server.modpacks.installed.missingFiles', { count: installed.missingFiles.length }).md()}
            </Text>
          </Alert>
        )}
        {installed.removedClientMods.length > 0 && (
          <Text size='xs' c='dimmed'>
            {tExt('pages.server.modpacks.installed.removedMods', { count: installed.removedClientMods.length })}
          </Text>
        )}
      </Stack>
    </Card>
  );
}
