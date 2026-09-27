import { faCodeBranch, faDownload, faFileLines, faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, Stack } from '@mantine/core';
import { useState } from 'react';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import ConditionalTooltip from '@/elements/overlays/ConditionalTooltip.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import { bytesToString } from '@/lib/format/size.ts';
import { useSearchablePaginatedTable } from '@/plugins/resource/useSearchablePaginatedTable.ts';
import { useServerStore } from '@/stores/server.ts';
import getVersions from '../../../api/server/getVersions.ts';
import LoaderBadge from '../../../components/LoaderBadge.tsx';
import ReleaseTypeBadge from '../../../components/ReleaseTypeBadge.tsx';
import { compactNumber } from '../../../lib/format.ts';
import { modpackQueryKeys } from '../../../lib/queryKeys.ts';
import {
  type Loader,
  loaderSchema,
  type ModpackDetails,
  type ModpackVersion,
  type Overview,
  type ReleaseType,
  releaseTypeSchema,
} from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';
import ChangelogModal from './ChangelogModal.tsx';

const PER_PAGE = 15;
const SHOWN_GAME_VERSIONS = 3;

interface Props {
  modpack: ModpackDetails;
  overview: Overview;
  onInstall: (version: ModpackVersion) => void;
}

export default function VersionsTab({ modpack, overview, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();

  const [loaderFilter, setLoaderFilter] = useState<Loader | null>(null);
  const [gameVersionFilter, setGameVersionFilter] = useState<string | null>(null);
  const [releaseType, setReleaseType] = useState<ReleaseType | null>(null);
  const [compatible, setCompatible] = useState(false);
  const [changelogVersion, setChangelogVersion] = useState<ModpackVersion | null>(null);

  const canMatch = !!overview.detected.loader || !!overview.detected.minecraftVersion;
  const loader = compatible ? overview.detected.loader : loaderFilter;
  const gameVersion = compatible ? overview.detected.minecraftVersion : gameVersionFilter;
  const installedVersionId =
    overview.installed?.source === modpack.provider && overview.installed.projectId === modpack.id
      ? overview.installed.versionId
      : null;

  const { data, loading, error, search, setSearch, setPage } = useSearchablePaginatedTable({
    queryKey: modpackQueryKeys.versions(server.uuid, modpack.provider, modpack.id),
    fetcher: (page, search) =>
      getVersions(server.uuid, modpack.provider, modpack.id, {
        page,
        perPage: PER_PAGE,
        search,
        loader,
        gameVersion,
        releaseType,
      }),
    paginationKey: 'versions',
    deps: [loader, gameVersion, releaseType],
    debounceMs: 250,
  });

  const changeFilter = <T,>(setter: (value: T) => void) => {
    return (value: T) => {
      setter(value);
      setPage(1);
    };
  };

  const loaderOptions = data?.loaders ?? loaderSchema.options;
  const gameVersionOptions = data?.gameVersions ?? [];

  return (
    <Stack gap='md'>
      <ChangelogModal
        provider={modpack.provider}
        projectId={modpack.id}
        version={changelogVersion}
        onClose={() => setChangelogVersion(null)}
      />

      <Group gap='sm' wrap='wrap'>
        <TextInput
          style={{ flex: 1, minWidth: 200 }}
          placeholder={tExt('pages.server.modpacks.versions.searchPlaceholder', {})}
          leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <Select
          w={170}
          clearable
          placeholder={tExt('pages.server.modpacks.versions.allLoaders', {})}
          value={loader}
          disabled={compatible}
          onChange={changeFilter((value: string | null) => setLoaderFilter(value as Loader | null))}
          data={(loader && !loaderOptions.includes(loader) ? [loader, ...loaderOptions] : loaderOptions).map(
            (value) => ({ value, label: tExt(`loaders.${value}`, {}) }),
          )}
        />
        <Select
          w={200}
          clearable
          searchable
          placeholder={tExt('pages.server.modpacks.versions.allGameVersions', {})}
          value={gameVersion}
          disabled={compatible}
          onChange={changeFilter(setGameVersionFilter)}
          data={
            gameVersion && !gameVersionOptions.includes(gameVersion)
              ? [gameVersion, ...gameVersionOptions]
              : gameVersionOptions
          }
        />
        <SegmentedControl
          value={releaseType ?? 'all'}
          onChange={changeFilter((value: string) => setReleaseType(value === 'all' ? null : (value as ReleaseType)))}
          data={[
            { value: 'all', label: tExt('pages.server.modpacks.versions.allReleaseTypes', {}) },
            ...releaseTypeSchema.options.map((value) => ({ value, label: tExt(`releaseTypes.${value}`, {}) })),
          ]}
        />
        {canMatch && (
          <Switch
            label={tExt('pages.server.modpacks.versions.compatible', {})}
            checked={compatible}
            onChange={(event) => changeFilter(setCompatible)(event.currentTarget.checked)}
          />
        )}
      </Group>

      <Table
        columns={[
          tExt('pages.server.modpacks.versions.columns.version', {}),
          tExt('pages.server.modpacks.versions.columns.type', {}),
          tExt('pages.server.modpacks.versions.columns.gameVersions', {}),
          tExt('pages.server.modpacks.versions.columns.loaders', {}),
          tExt('pages.server.modpacks.versions.columns.published', {}),
          tExt('pages.server.modpacks.versions.columns.downloads', {}),
          tExt('pages.server.modpacks.versions.columns.size', {}),
          '',
        ]}
        loading={loading}
        error={error}
        pagination={data?.versions}
        onPageSelect={setPage}
        empty={
          <EmptyState
            flush
            icon={faCodeBranch}
            title={tExt('pages.server.modpacks.versions.empty', {})}
            description=''
          />
        }
      >
        {data?.versions.data.map((version) => (
          <TableRow key={version.id}>
            <TableData>
              <Stack gap={2}>
                <Group gap={6} wrap='nowrap'>
                  <Text fw={500} size='sm'>
                    {version.name}
                  </Text>
                  {version.id === installedVersionId && (
                    <Badge color='green' variant='filled' size='xs'>
                      {tExt('pages.server.modpacks.versions.installed', {})}
                    </Badge>
                  )}
                </Group>
                {version.versionNumber !== version.name && (
                  <Text size='xs' c='dimmed' ff='monospace'>
                    {version.versionNumber}
                  </Text>
                )}
              </Stack>
            </TableData>
            <TableData>
              <ReleaseTypeBadge releaseType={version.releaseType} />
            </TableData>
            <TableData>
              <Group gap={4} wrap='nowrap'>
                {version.gameVersions.slice(0, SHOWN_GAME_VERSIONS).map((value) => (
                  <Badge key={value} color='gray' variant='light' size='sm'>
                    {value}
                  </Badge>
                ))}
                {version.gameVersions.length > SHOWN_GAME_VERSIONS && (
                  <Tooltip label={version.gameVersions.slice(SHOWN_GAME_VERSIONS).join(', ')}>
                    <Badge color='gray' variant='outline' size='sm'>
                      {tExt('pages.server.modpacks.versions.moreGameVersions', {
                        count: version.gameVersions.length - SHOWN_GAME_VERSIONS,
                      })}
                    </Badge>
                  </Tooltip>
                )}
              </Group>
            </TableData>
            <TableData>
              <Group gap={4} wrap='nowrap'>
                {version.loaders.map((value) => (
                  <LoaderBadge key={value} loader={value} />
                ))}
              </Group>
            </TableData>
            <TableData>
              <FormattedTimestamp timestamp={version.published} />
            </TableData>
            <TableData>{compactNumber(version.downloads)}</TableData>
            <TableData>{version.fileSize > 0 ? bytesToString(version.fileSize) : '—'}</TableData>
            <TableData>
              <Group gap='xs' justify='flex-end' wrap='nowrap'>
                <Tooltip label={tExt('pages.server.modpacks.versions.changelog', {})}>
                  <ActionIcon variant='subtle' color='gray' onClick={() => setChangelogVersion(version)}>
                    <FontAwesomeIcon icon={faFileLines} />
                  </ActionIcon>
                </Tooltip>
                <ServerCan action='modpacks.install'>
                  <ConditionalTooltip
                    enabled={!version.downloadable}
                    label={tExt('pages.server.modpacks.versions.notDownloadable', {})}
                  >
                    <Button
                      size='xs'
                      variant='light'
                      disabled={!version.downloadable}
                      leftSection={<FontAwesomeIcon icon={faDownload} />}
                      onClick={() => onInstall(version)}
                    >
                      {tExt('pages.server.modpacks.versions.install', {})}
                    </Button>
                  </ConditionalTooltip>
                </ServerCan>
              </Group>
            </TableData>
          </TableRow>
        ))}
      </Table>
    </Stack>
  );
}
