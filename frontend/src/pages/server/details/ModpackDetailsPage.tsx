import { faDiscord } from '@fortawesome/free-brands-svg-icons';
import {
  faArrowLeft,
  faArrowUpRightFromSquare,
  faBook,
  faBug,
  faCalendar,
  faClock,
  faCode,
  faDownload,
  faGlobe,
  faHandHoldingHeart,
  faHeart,
  faScaleBalanced,
  faTriangleExclamation,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Anchor, Center, Divider, Group, Stack } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import Title from '@/elements/typography/Title.tsx';
import { useServerStore } from '@/stores/server.ts';
import getModpack from '../../../api/server/getModpack.ts';
import LoaderBadge from '../../../components/LoaderBadge.tsx';
import ModpackIcon from '../../../components/ModpackIcon.tsx';
import ProviderMark from '../../../components/ProviderMark.tsx';
import { compactNumber } from '../../../lib/format.ts';
import { modpackQueryKeys } from '../../../lib/queryKeys.ts';
import { type ModpackDetails, type Overview, providerSchema } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';
import { type InstallTarget, presetFromVersion } from '../modals/InstallModpackModal.tsx';
import GalleryTab from './GalleryTab.tsx';
import RichText from './RichText.tsx';
import VersionsTab from './VersionsTab.tsx';

const TABS = ['description', 'versions', 'gallery'] as const;
type Tab = (typeof TABS)[number];

const LINK_ICONS = {
  website: faGlobe,
  source: faCode,
  issues: faBug,
  wiki: faBook,
  discord: faDiscord,
  donation: faHandHoldingHeart,
} as const;
type LinkKind = keyof typeof LINK_ICONS;

interface Props {
  overview: Overview;
  onInstall: (target: InstallTarget) => void;
}

function Stat({ icon, label, children }: { icon: typeof faGlobe; label: string; children: ReactNode }) {
  return (
    <Stack gap={0}>
      <Text size='xs' c='dimmed' tt='uppercase' fw={600}>
        <FontAwesomeIcon icon={icon} /> {label}
      </Text>
      <Text size='sm' fw={500} component='div'>
        {children}
      </Text>
    </Stack>
  );
}

export default function ModpackDetailsPage({ overview, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams();

  const provider = providerSchema.safeParse(params.provider).data;
  const projectId = params.project ?? '';
  const tab: Tab = TABS.find((value) => value === params.tab) ?? 'description';
  const basePath = `/server/${server.uuidShort}/modpacks`;

  const details = useQuery({
    queryKey: modpackQueryKeys.project(server.uuid, provider ?? '', projectId),
    queryFn: () => getModpack(server.uuid, provider!, projectId),
    enabled: !!provider && !!projectId,
  });

  const goBack = () => {
    if (location.key !== 'default') navigate(-1);
    else navigate(basePath);
  };

  const backButton = (
    <Group>
      <Button variant='subtle' color='gray' leftSection={<FontAwesomeIcon icon={faArrowLeft} />} onClick={goBack}>
        {tExt('pages.server.modpacks.details.back', {})}
      </Button>
    </Group>
  );

  if (!provider || details.error) {
    return (
      <Stack gap='md'>
        {backButton}
        <Alert color='red'>
          {details.error ? httpErrorToHuman(details.error) : tExt('pages.server.modpacks.details.unknownProvider', {})}
        </Alert>
      </Stack>
    );
  }

  if (!details.data) {
    return (
      <Stack gap='md'>
        {backButton}
        <Center py='xl'>
          <Spinner />
        </Center>
      </Stack>
    );
  }

  const modpack: ModpackDetails = details.data;
  const installed =
    overview.installed?.source === modpack.provider && overview.installed.projectId === modpack.id
      ? overview.installed
      : null;
  const target: InstallTarget = {
    provider: modpack.provider,
    projectId: modpack.id,
    name: modpack.name,
    iconUrl: modpack.iconUrl,
    clientOnly: modpack.clientOnly,
  };

  return (
    <Stack gap='md'>
      {backButton}

      <Card p='lg'>
        <Group justify='space-between' align='flex-start' wrap='wrap' gap='lg'>
          <Group wrap='nowrap' align='flex-start' gap='lg' style={{ minWidth: 0, flex: 1 }}>
            <ModpackIcon url={modpack.iconUrl} name={modpack.name} size={96} />
            <Stack gap={6} style={{ minWidth: 0 }}>
              <Group gap='sm'>
                <Title order={2}>{modpack.name}</Title>
                {installed && (
                  <Badge color='green' variant='filled'>
                    {tExt('pages.server.modpacks.versions.installed', {})}
                  </Badge>
                )}
              </Group>
              <Group gap='xs'>
                {modpack.authors.length > 0 && (
                  <Text size='sm' c='dimmed'>
                    {tExt('pages.server.modpacks.details.by', { authors: modpack.authors.join(', ') })}
                  </Text>
                )}
                <Text size='sm' c='dimmed' component='div'>
                  <ProviderMark provider={modpack.provider} />
                </Text>
              </Group>
              <Text>{modpack.summary}</Text>
              <Group gap={6}>
                {modpack.loaders.map((loader) => (
                  <LoaderBadge key={loader} loader={loader} />
                ))}
                {modpack.categories.map((category) => (
                  <Badge key={category} color='gray' variant='outline' size='sm'>
                    {category}
                  </Badge>
                ))}
              </Group>
            </Stack>
          </Group>

          <Stack gap='xs' align='stretch'>
            <ServerCan action='modpacks.install'>
              <Button
                size='md'
                color='blue'
                leftSection={<FontAwesomeIcon icon={faDownload} />}
                onClick={() => onInstall(target)}
              >
                {tExt('pages.server.modpacks.details.install', {})}
              </Button>
            </ServerCan>
            <Anchor href={modpack.url} target='_blank' rel='noopener noreferrer' size='sm' ta='center'>
              <FontAwesomeIcon icon={faArrowUpRightFromSquare} />{' '}
              {tExt('pages.server.modpacks.details.openOn', { provider: tExt(`providers.${modpack.provider}`, {}) })}
            </Anchor>
          </Stack>
        </Group>

        <Divider my='md' />

        <Group justify='space-between' wrap='wrap' gap='lg'>
          <Group gap='xl' wrap='wrap'>
            <Stat icon={faDownload} label={tExt('pages.server.modpacks.details.stats.downloads', {})}>
              {compactNumber(modpack.downloads)}
            </Stat>
            <Stat icon={faHeart} label={tExt('pages.server.modpacks.details.stats.follows', {})}>
              {compactNumber(modpack.follows)}
            </Stat>
            {modpack.created && (
              <Stat icon={faCalendar} label={tExt('pages.server.modpacks.details.stats.created', {})}>
                <FormattedTimestamp timestamp={modpack.created} />
              </Stat>
            )}
            {modpack.updated && (
              <Stat icon={faClock} label={tExt('pages.server.modpacks.details.stats.updated', {})}>
                <FormattedTimestamp timestamp={modpack.updated} />
              </Stat>
            )}
            {modpack.license && (
              <Stat icon={faScaleBalanced} label={tExt('pages.server.modpacks.details.stats.license', {})}>
                {modpack.license}
              </Stat>
            )}
          </Group>
          {modpack.links.length > 0 && (
            <Group gap='sm'>
              {modpack.links.map((link) => {
                const kind = (Object.keys(LINK_ICONS) as LinkKind[]).find((value) => value === link.kind) ?? 'website';

                return (
                  <Anchor
                    key={`${link.kind}-${link.url}`}
                    href={link.url}
                    target='_blank'
                    rel='noopener noreferrer'
                    size='sm'
                  >
                    <FontAwesomeIcon icon={LINK_ICONS[kind]} />{' '}
                    {tExt(`pages.server.modpacks.details.links.${kind}`, {})}
                  </Anchor>
                );
              })}
            </Group>
          )}
        </Group>
      </Card>

      {modpack.clientOnly && (
        <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />}>
          <Text size='sm' component='div'>
            {tExt('pages.server.modpacks.details.clientOnly', {}).md()}
          </Text>
        </Alert>
      )}
      {installed && (
        <Alert color='green'>
          {tExt('pages.server.modpacks.details.installedHere', { version: installed.versionName })}
        </Alert>
      )}

      <Tabs
        value={tab}
        onChange={(value) =>
          navigate(
            `${basePath}/${modpack.provider}/${projectId}${value && value !== 'description' ? `/${value}` : ''}`,
            {
              replace: true,
            },
          )
        }
      >
        <Tabs.List>
          <Tabs.Tab value='description'>{tExt('pages.server.modpacks.details.tabs.description', {})}</Tabs.Tab>
          <Tabs.Tab value='versions'>{tExt('pages.server.modpacks.details.tabs.versions', {})}</Tabs.Tab>
          {modpack.gallery.length > 0 && (
            <Tabs.Tab
              value='gallery'
              rightSection={
                <Badge size='xs' variant='light' color='gray'>
                  {modpack.gallery.length}
                </Badge>
              }
            >
              {tExt('pages.server.modpacks.details.tabs.gallery', {})}
            </Tabs.Tab>
          )}
        </Tabs.List>

        <Tabs.Panel value='description' pt='md'>
          <Card p='lg'>
            {modpack.body.trim() ? (
              <RichText content={modpack.body} format={modpack.bodyFormat} />
            ) : (
              <Text c='dimmed'>{tExt('pages.server.modpacks.details.emptyDescription', {})}</Text>
            )}
          </Card>
        </Tabs.Panel>
        <Tabs.Panel value='versions' pt='md'>
          {tab === 'versions' && (
            <VersionsTab
              modpack={modpack}
              overview={overview}
              onInstall={(version) => onInstall({ ...target, preset: presetFromVersion(version, overview.detected) })}
            />
          )}
        </Tabs.Panel>
        <Tabs.Panel value='gallery' pt='md'>
          <GalleryTab gallery={modpack.gallery} />
        </Tabs.Panel>
      </Tabs>
    </Stack>
  );
}
