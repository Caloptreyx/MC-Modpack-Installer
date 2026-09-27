import { faBoxOpen, faFilterCircleXmark, faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, SimpleGrid, Skeleton, Stack } from '@mantine/core';
import { useDebouncedValue } from '@mantine/hooks';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Card from '@/elements/data-display/Card.tsx';
import { Pagination } from '@/elements/data-display/Table.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import ConditionalTooltip from '@/elements/overlays/ConditionalTooltip.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import getFilters from '../../../api/server/getFilters.ts';
import searchModpacks from '../../../api/server/searchModpacks.ts';
import ProviderMark from '../../../components/ProviderMark.tsx';
import { modpackQueryKeys } from '../../../lib/queryKeys.ts';
import {
  type Loader,
  loaderSchema,
  type Overview,
  type Provider,
  providerSchema,
  type SortMode,
  sortModeSchema,
} from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';
import type { InstallTarget } from '../modals/InstallModpackModal.tsx';
import InstalledModpackCard from './InstalledModpackCard.tsx';
import ModpackCard from './ModpackCard.tsx';

const PER_PAGE = 18;

interface Props {
  overview: Overview;
  onInstall: (target: InstallTarget) => void;
}

export default function BrowseModpacksPage({ overview, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();

  const available = (provider: Provider) => overview.providers[provider];
  const requestedProvider = providerSchema.safeParse(params.get('provider')).data;
  const provider: Provider =
    requestedProvider && available(requestedProvider)
      ? requestedProvider
      : overview.providers.modrinth
        ? 'modrinth'
        : 'curseforge';

  const matchServer = params.get('match') === '1';
  const canMatch = !!overview.detected.loader || !!overview.detected.minecraftVersion;
  const loader: Loader | null = matchServer
    ? overview.detected.loader
    : (loaderSchema.safeParse(params.get('loader')).data ?? null);
  const gameVersion = matchServer ? overview.detected.minecraftVersion : params.get('version');
  const category = params.get('category');
  const hideClientOnly = params.get('clientOnly') !== '1';
  const page = Math.max(1, Number.parseInt(params.get('page') ?? '1', 10) || 1);
  const query = params.get('q') ?? '';
  // relevance only means something for a search term, otherwise show the most popular packs
  const sort: SortMode = sortModeSchema.safeParse(params.get('sort')).data ?? (query ? 'relevance' : 'downloads');

  const [search, setSearch] = useState(query);
  const [debouncedSearch] = useDebouncedValue(search, 350);

  const update = (patch: Record<string, string | null>, resetPage = true) => {
    setParams(
      (current) => {
        const next = new URLSearchParams(current);
        for (const [key, value] of Object.entries(patch)) {
          if (value === null || value === '') next.delete(key);
          else next.set(key, value);
        }
        if (resetPage) next.delete('page');
        return next;
      },
      { replace: true },
    );
  };

  useEffect(() => {
    if (debouncedSearch !== query) update({ q: debouncedSearch });
  }, [debouncedSearch]);

  const filters = useQuery({
    queryKey: modpackQueryKeys.filters(server.uuid, provider),
    queryFn: () => getFilters(server.uuid, provider),
    staleTime: 60 * 60 * 1000,
  });

  const results = useQuery({
    queryKey: [
      ...modpackQueryKeys.search(server.uuid),
      { provider, query, loader, gameVersion, category, sort, hideClientOnly, page },
    ],
    queryFn: () =>
      searchModpacks(server.uuid, {
        provider,
        page,
        perPage: PER_PAGE,
        search: query,
        loader,
        gameVersion,
        category,
        sort,
        hideClientOnly: provider === 'modrinth' && hideClientOnly,
      }),
    placeholderData: keepPreviousData,
  });

  const hasFilters = !!(query || loader || gameVersion || category || matchServer);
  const installed = overview.installed;

  return (
    <Stack gap='md'>
      {installed && (
        <InstalledModpackCard
          installed={installed}
          providerAvailable={available(installed.source)}
          onView={() => navigate(`/server/${server.uuidShort}/modpacks/${installed.source}/${installed.projectId}`)}
          onInstall={(preset) =>
            onInstall({
              provider: installed.source,
              projectId: installed.projectId,
              name: installed.name,
              iconUrl: installed.iconUrl,
              clientOnly: false,
              preset,
            })
          }
        />
      )}

      <Card p='md'>
        <Stack gap='sm'>
          <Group gap='sm' wrap='wrap'>
            <SegmentedControl
              value={provider}
              onChange={(value) => update({ provider: value, category: null, version: null })}
              data={(['modrinth', 'curseforge'] as const).map((value) => ({
                value,
                disabled: !available(value),
                label: (
                  <ConditionalTooltip
                    enabled={value === 'curseforge' && !available(value)}
                    label={tExt('pages.server.modpacks.browse.curseforgeUnavailable', {})}
                  >
                    <ProviderMark provider={value} />
                  </ConditionalTooltip>
                ),
              }))}
            />
            <TextInput
              style={{ flex: 1, minWidth: 220 }}
              placeholder={tExt('pages.server.modpacks.browse.searchPlaceholder', {})}
              leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
            <Select
              w={190}
              aria-label={tExt('pages.server.modpacks.browse.sort', {})}
              value={sort}
              onChange={(value) => update({ sort: value })}
              data={sortModeSchema.options.map((value) => ({
                value,
                label: tExt(`pages.server.modpacks.browse.sorts.${value}`, {}),
              }))}
            />
          </Group>

          <Group gap='sm' wrap='wrap' align='center'>
            <Select
              w={170}
              clearable
              aria-label={tExt('pages.server.modpacks.browse.loader', {})}
              placeholder={tExt('pages.server.modpacks.browse.anyLoader', {})}
              value={loader}
              disabled={matchServer}
              onChange={(value) => update({ loader: value })}
              data={(filters.data?.loaders ?? loaderSchema.options).map((value) => ({
                value,
                label: tExt(`loaders.${value}`, {}),
              }))}
            />
            <Select
              w={170}
              clearable
              searchable
              aria-label={tExt('pages.server.modpacks.browse.gameVersion', {})}
              placeholder={tExt('pages.server.modpacks.browse.anyVersion', {})}
              value={gameVersion}
              disabled={matchServer}
              onChange={(value) => update({ version: value })}
              data={
                gameVersion && !filters.data?.gameVersions.includes(gameVersion)
                  ? [gameVersion, ...(filters.data?.gameVersions ?? [])]
                  : (filters.data?.gameVersions ?? [])
              }
            />
            <Select
              w={190}
              clearable
              searchable
              aria-label={tExt('pages.server.modpacks.browse.category', {})}
              placeholder={tExt('pages.server.modpacks.browse.anyCategory', {})}
              value={category}
              onChange={(value) => update({ category: value })}
              data={(filters.data?.categories ?? []).map((item) => ({ value: item.id, label: item.name }))}
            />
            {canMatch && (
              <ConditionalTooltip
                enabled
                label={tExt('pages.server.modpacks.browse.matchServerHint', {
                  loader: overview.detected.loader ? tExt(`loaders.${overview.detected.loader}`, {}) : '—',
                  version: overview.detected.minecraftVersion ?? '—',
                })}
              >
                <Switch
                  label={tExt('pages.server.modpacks.browse.matchServer', {})}
                  checked={matchServer}
                  onChange={(event) => update({ match: event.currentTarget.checked ? '1' : null })}
                />
              </ConditionalTooltip>
            )}
            {provider === 'modrinth' && (
              <Switch
                label={tExt('pages.server.modpacks.browse.hideClientOnly', {})}
                checked={hideClientOnly}
                onChange={(event) => update({ clientOnly: event.currentTarget.checked ? null : '1' })}
              />
            )}
            {hasFilters && (
              <Button
                variant='subtle'
                color='gray'
                leftSection={<FontAwesomeIcon icon={faFilterCircleXmark} />}
                onClick={() => {
                  setSearch('');
                  update({ q: null, loader: null, version: null, category: null, match: null });
                }}
              >
                {tExt('pages.server.modpacks.browse.clearFilters', {})}
              </Button>
            )}
          </Group>
        </Stack>
      </Card>

      {results.error ? (
        <Alert color='red'>{httpErrorToHuman(results.error)}</Alert>
      ) : !results.data ? (
        <SimpleGrid cols={{ base: 1, md: 2, xl: 3 }}>
          {Array.from({ length: 6 }, (_, index) => (
            <Skeleton key={index} height={176} radius='md' />
          ))}
        </SimpleGrid>
      ) : results.data.total === 0 ? (
        <EmptyState
          icon={faBoxOpen}
          title={tExt('pages.server.modpacks.browse.empty.title', {})}
          description={tExt('pages.server.modpacks.browse.empty.description', {})}
        />
      ) : (
        <>
          <Group justify='space-between'>
            <Text size='sm' c='dimmed'>
              {tExt('pages.server.modpacks.browse.results', { count: results.data.total.toLocaleString() })}
            </Text>
            <Pagination data={results.data} onPageSelect={(next) => update({ page: String(next) }, false)} />
          </Group>
          <SimpleGrid cols={{ base: 1, md: 2, xl: 3 }} style={{ opacity: results.isPlaceholderData ? 0.6 : 1 }}>
            {results.data.data.map((modpack) => (
              <ModpackCard
                key={`${modpack.provider}-${modpack.id}`}
                modpack={modpack}
                installed={installed?.source === modpack.provider && installed.projectId === modpack.id}
                onOpen={() => navigate(`/server/${server.uuidShort}/modpacks/${modpack.provider}/${modpack.id}`)}
                onInstall={() =>
                  onInstall({
                    provider: modpack.provider,
                    projectId: modpack.id,
                    name: modpack.name,
                    iconUrl: modpack.iconUrl,
                    clientOnly: modpack.clientOnly,
                  })
                }
              />
            ))}
          </SimpleGrid>
          <Group justify='flex-end'>
            <Pagination data={results.data} onPageSelect={(next) => update({ page: String(next) }, false)} />
          </Group>
        </>
      )}
    </Stack>
  );
}
