import { faDownload, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, SimpleGrid, Stack } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Checkbox from '@/elements/input/Checkbox.tsx';
import Select from '@/elements/input/Select.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import getVersions from '../../../api/server/getVersions.ts';
import installModpack from '../../../api/server/installModpack.ts';
import ModpackIcon from '../../../components/ModpackIcon.tsx';
import ProviderMark from '../../../components/ProviderMark.tsx';
import { recommendedImage, requiredJava } from '../../../lib/java.ts';
import { modpackQueryKeys } from '../../../lib/queryKeys.ts';
import type { InstallMode, Loader, ModpackVersion, Overview, Provider } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

export interface InstallTarget {
  provider: Provider;
  projectId: string;
  name: string;
  iconUrl: string | null;
  clientOnly: boolean;
  /** Preselected loader, Minecraft version and version; detection-based defaults fill the rest. */
  preset?: InstallPreset;
}

export interface InstallPreset {
  loader: Loader | null;
  gameVersion: string | null;
  versionId: string | null;
}

/** Preset for installing `version`, preferring the detected loader and Minecraft version when it supports them. */
export function presetFromVersion(version: ModpackVersion, detected: Overview['detected']): InstallPreset {
  return {
    loader: prefer(version.loaders, detected.loader),
    gameVersion: prefer(version.gameVersions, detected.minecraftVersion),
    versionId: version.id,
  };
}

interface Props {
  opened: boolean;
  onClose: () => void;
  target: InstallTarget | null;
  overview: Overview;
}

/** `detected` when the list offers it, otherwise `fallback`, otherwise the first entry. */
function prefer<T>(list: T[], detected: T | null, fallback?: T | null): T | null {
  if (detected !== null && list.includes(detected)) return detected;
  if (fallback !== undefined && fallback !== null && list.includes(fallback)) return fallback;
  return list[0] ?? null;
}

function defaultVersion(versions: ModpackVersion[]): ModpackVersion | null {
  const downloadable = versions.filter((version) => version.downloadable);
  return downloadable.find((version) => version.releaseType === 'release') ?? downloadable[0] ?? versions[0] ?? null;
}

export default function InstallModpackModal({ opened, onClose, target, overview }: Props) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const navigate = useNavigate();
  const { server, updateServer } = useServerStore();
  const canChangeImagePermission = useServerCan('startup.docker-image');

  const [loader, setLoader] = useState<Loader | null>(null);
  const [gameVersion, setGameVersion] = useState<string | null>(null);
  const [versionId, setVersionId] = useState<string | null>(null);
  const [mode, setMode] = useState<InstallMode>('replace');
  const [acceptEula, setAcceptEula] = useState(false);
  const [dockerImage, setDockerImage] = useState<string | null>(null);
  const [imageTouched, setImageTouched] = useState(false);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!opened || !target) return;

    setLoader(target.preset?.loader ?? null);
    setGameVersion(target.preset?.gameVersion ?? null);
    setVersionId(target.preset?.versionId ?? null);
    setMode('replace');
    setAcceptEula(false);
    setDockerImage(null);
    setImageTouched(false);
  }, [opened, target]);

  const enabled = opened && !!target;
  const versionsKey = target ? modpackQueryKeys.versions(server.uuid, target.provider, target.projectId) : [];

  // Unfiltered: every loader of the pack and its newest version.
  const facets = useQuery({
    queryKey: [...versionsKey, 'facets'],
    queryFn: () => getVersions(server.uuid, target!.provider, target!.projectId, { page: 1, perPage: 1 }),
    enabled,
  });

  // Filtered by loader (+ Minecraft version): the Minecraft versions of the loader and the matching pack versions.
  const filtered = useQuery({
    queryKey: [...versionsKey, 'install', loader, gameVersion],
    queryFn: () =>
      getVersions(server.uuid, target!.provider, target!.projectId, {
        page: 1,
        perPage: 100,
        loader,
        gameVersion,
      }),
    // wait for the loader default so the first request already uses it
    enabled: enabled && !!facets.data && (loader !== null || facets.data.loaders.length === 0),
  });

  // Default the loader to the detected one, else to the loader of the newest version.
  useEffect(() => {
    if (!facets.data) return;

    const wanted =
      loader && facets.data.loaders.includes(loader)
        ? loader
        : prefer(facets.data.loaders, overview.detected.loader, facets.data.versions.data[0]?.loaders[0]);
    if (wanted !== loader) setLoader(wanted);
  }, [facets.data, loader]);

  // Default the Minecraft version to the detected one, else the newest, then pick a pack version.
  useEffect(() => {
    const data = filtered.data;
    if (!data) return;

    const wantedGameVersion =
      gameVersion && data.gameVersions.includes(gameVersion)
        ? gameVersion
        : prefer(data.gameVersions, overview.detected.minecraftVersion);
    if (wantedGameVersion !== gameVersion) {
      setGameVersion(wantedGameVersion);
      return;
    }

    const versions = data.versions.data;
    if (!versionId || !versions.some((version) => version.id === versionId)) {
      setVersionId(defaultVersion(versions)?.id ?? null);
    }
  }, [filtered.data, gameVersion, versionId]);

  const versions = filtered.data?.versions.data ?? [];
  const selectedVersion = versions.find((version) => version.id === versionId) ?? null;
  const minecraftVersion = gameVersion ?? selectedVersion?.gameVersions[0] ?? null;

  const java = minecraftVersion ? requiredJava(minecraftVersion) : null;
  const canChangeImage = overview.canChangeImage && canChangeImagePermission && overview.dockerImages.length > 1;
  const recommended = java ? recommendedImage(overview.dockerImages, overview.currentImage, java) : null;
  const chosenImage = overview.dockerImages.find(
    (image) => image.image === (canChangeImage ? (dockerImage ?? overview.currentImage) : overview.currentImage),
  );

  useEffect(() => {
    if (!imageTouched) setDockerImage(recommended?.image ?? overview.currentImage);
  }, [recommended?.image, imageTouched, overview.currentImage]);

  const imageMismatch =
    java !== null &&
    chosenImage?.javaVersion != null &&
    (java === 8 ? chosenImage.javaVersion !== 8 : chosenImage.javaVersion < java);

  const loaderOptions = useMemo(
    () =>
      (facets.data?.loaders ?? []).map((value) => {
        const label = tExt(`loaders.${value}`, {});
        return {
          value,
          label:
            value === overview.detected.loader
              ? tExt('pages.server.modpacks.install.detectedOption', { value: label })
              : label,
        };
      }),
    [facets.data, overview.detected.loader],
  );

  const gameVersionOptions = useMemo(
    () =>
      (filtered.data?.gameVersions ?? (gameVersion ? [gameVersion] : [])).map((value) => ({
        value,
        label:
          value === overview.detected.minecraftVersion
            ? tExt('pages.server.modpacks.install.detectedOption', { value })
            : value,
      })),
    [filtered.data, gameVersion, overview.detected.minecraftVersion],
  );

  const versionOptions = versions.map((version) => ({
    value: version.id,
    label: tExt('pages.server.modpacks.install.versionOption', {
      name: version.name,
      type: tExt(`releaseTypes.${version.releaseType}`, {}),
    }),
    disabled: !version.downloadable,
  }));

  const imageOptions = overview.dockerImages.map((image) => ({
    value: image.image,
    label:
      image.image === overview.currentImage
        ? tExt('pages.server.modpacks.install.dockerImageCurrent', { name: image.name })
        : image.image === recommended?.image
          ? tExt('pages.server.modpacks.install.dockerImageRecommended', { name: image.name })
          : image.name,
  }));

  const loaderChanged = !!overview.detected.loader && !!loader && overview.detected.loader !== loader;
  const versionChanged =
    !!overview.detected.minecraftVersion &&
    !!minecraftVersion &&
    overview.detected.minecraftVersion !== minecraftVersion;
  const resolving = facets.isLoading || filtered.isLoading || (filtered.isFetching && !selectedVersion);

  const doInstall = async () => {
    if (!target || !selectedVersion) return;

    setLoading(true);
    try {
      await installModpack(server.uuid, {
        provider: target.provider,
        projectId: target.projectId,
        versionId: selectedVersion.id,
        mode,
        acceptEula,
        dockerImage: canChangeImage && dockerImage && dockerImage !== overview.currentImage ? dockerImage : undefined,
      });

      addToast(tExt('pages.server.modpacks.install.toast', { name: target.name }), 'success');
      onClose();
      navigate(`/server/${server.uuidShort}`);
      updateServer({ status: 'installing' });
    } catch (error) {
      addToast(httpErrorToHuman(error), 'error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      size='lg'
      title={
        target ? (
          <Group gap='sm' wrap='nowrap'>
            <ModpackIcon url={target.iconUrl} name={target.name} size={36} />
            <Stack gap={0}>
              <Text fw={600}>{tExt('pages.server.modpacks.install.title', { name: target.name })}</Text>
              <Text size='xs' c='dimmed' component='div'>
                <ProviderMark provider={target.provider} />
              </Text>
            </Stack>
          </Group>
        ) : null
      }
    >
      <Stack gap='md'>
        <SimpleGrid cols={{ base: 1, sm: 2 }}>
          <Select
            withAsterisk
            label={tExt('pages.server.modpacks.install.loader', {})}
            data={loaderOptions}
            value={loader}
            onChange={(value) => {
              setLoader(value as Loader | null);
              setGameVersion(null);
              setVersionId(null);
            }}
            disabled={facets.isLoading || loaderOptions.length === 0}
            placeholder={loaderOptions.length === 0 && facets.isSuccess ? tExt('loaders.unknown', {}) : undefined}
          />
          <Select
            withAsterisk
            searchable
            label={tExt('pages.server.modpacks.install.gameVersion', {})}
            data={gameVersionOptions}
            value={gameVersion}
            onChange={(value) => {
              setGameVersion(value);
              setVersionId(null);
            }}
            disabled={!filtered.data || gameVersionOptions.length === 0}
          />
        </SimpleGrid>

        <Select
          withAsterisk
          searchable
          label={tExt('pages.server.modpacks.install.version', {})}
          data={versionOptions}
          value={versionId}
          onChange={setVersionId}
          disabled={!filtered.data || versionOptions.length === 0}
          error={
            filtered.data && versionOptions.length === 0
              ? tExt('pages.server.modpacks.install.noVersions', {})
              : undefined
          }
        />

        <Stack gap={6}>
          <Text size='sm' fw={500}>
            {tExt('pages.server.modpacks.install.mode', {})}
          </Text>
          <SegmentedControl
            fullWidth
            value={mode}
            onChange={(value) => setMode(value as InstallMode)}
            data={[
              { value: 'replace', label: tExt('pages.server.modpacks.install.modes.replace', {}) },
              {
                value: 'wipe',
                label: tExt('pages.server.modpacks.install.modes.wipe', {}),
                disabled: !overview.allowCleanInstall,
              },
            ]}
          />
          <Text size='xs' c={mode === 'wipe' ? 'red' : 'dimmed'} component='div'>
            {mode === 'wipe'
              ? tExt('pages.server.modpacks.install.modes.wipeDescription', {}).md()
              : overview.allowCleanInstall
                ? tExt('pages.server.modpacks.install.modes.replaceDescription', {})
                : `${tExt('pages.server.modpacks.install.modes.replaceDescription', {})} ${tExt('pages.server.modpacks.install.modes.wipeDisabled', {})}`}
          </Text>
        </Stack>

        {canChangeImage && (
          <Select
            label={tExt('pages.server.modpacks.install.dockerImage', {})}
            description={
              java && minecraftVersion
                ? tExt('pages.server.modpacks.install.dockerImageDescription', { version: minecraftVersion, java })
                : undefined
            }
            data={imageOptions}
            value={dockerImage ?? overview.currentImage}
            onChange={(value) => {
              setImageTouched(true);
              setDockerImage(value);
            }}
          />
        )}

        <Checkbox
          checked={acceptEula}
          onChange={(event) => setAcceptEula(event.currentTarget.checked)}
          label={tExt('pages.server.modpacks.install.acceptEula', {}).md()}
        />

        {(loaderChanged ||
          versionChanged ||
          target?.clientOnly ||
          imageMismatch ||
          selectedVersion?.downloadable === false) && (
          <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />}>
            <Stack gap={4}>
              {target?.clientOnly && (
                <Text size='sm'>{tExt('pages.server.modpacks.install.warnings.clientOnly', {})}</Text>
              )}
              {selectedVersion?.downloadable === false && (
                <Text size='sm'>{tExt('pages.server.modpacks.install.warnings.notDownloadable', {})}</Text>
              )}
              {loaderChanged && loader && overview.detected.loader && (
                <Text size='sm' component='div'>
                  {tExt('pages.server.modpacks.install.warnings.loaderChange', {
                    current: tExt(`loaders.${overview.detected.loader}`, {}),
                    next: tExt(`loaders.${loader}`, {}),
                  }).md()}
                </Text>
              )}
              {versionChanged && overview.detected.minecraftVersion && minecraftVersion && (
                <Text size='sm' component='div'>
                  {tExt('pages.server.modpacks.install.warnings.versionChange', {
                    current: overview.detected.minecraftVersion,
                    next: minecraftVersion,
                  }).md()}
                </Text>
              )}
              {imageMismatch && java && minecraftVersion && chosenImage?.javaVersion != null && (
                <Text size='sm'>
                  {tExt('pages.server.modpacks.install.dockerImageMismatch', {
                    current: chosenImage.javaVersion,
                    version: minecraftVersion,
                    java,
                  })}
                </Text>
              )}
            </Stack>
          </Alert>
        )}

        <Text size='xs' c='dimmed'>
          {tExt('pages.server.modpacks.install.notice', {})}
        </Text>

        <ModalFooter>
          <Button
            color={mode === 'wipe' ? 'red' : 'blue'}
            leftSection={<FontAwesomeIcon icon={faDownload} />}
            loading={loading || resolving}
            disabled={!selectedVersion || !selectedVersion.downloadable}
            onClick={doInstall}
          >
            {tExt('pages.server.modpacks.install.submit', {})}
          </Button>
          <Button variant='default' onClick={onClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </Modal>
  );
}
