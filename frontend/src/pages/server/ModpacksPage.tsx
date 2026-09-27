import { faPlug } from '@fortawesome/free-solid-svg-icons';
import { Center } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { Route, Routes } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import { useServerStore } from '@/stores/server.ts';
import getOverview from '../../api/server/getOverview.ts';
import { modpackQueryKeys } from '../../lib/queryKeys.ts';
import { useExtTranslations } from '../../translations.ts';
import BrowseModpacksPage from './browse/BrowseModpacksPage.tsx';
import DetectedSetup from './DetectedSetup.tsx';
import ModpackDetailsPage from './details/ModpackDetailsPage.tsx';
import InstallModpackModal, { type InstallTarget } from './modals/InstallModpackModal.tsx';

/** Server route `/modpacks/*`: the browser and the modpack details pages share the overview and install modal. */
export default function ModpacksPage() {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();
  const [installTarget, setInstallTarget] = useState<InstallTarget | null>(null);

  const overview = useQuery({
    queryKey: modpackQueryKeys.overview(server.uuid),
    queryFn: () => getOverview(server.uuid),
  });

  const noProviders = overview.data && !overview.data.providers.modrinth && !overview.data.providers.curseforge;

  return (
    <ServerContentContainer
      title={tExt('pages.server.modpacks.title', {})}
      contentRight={overview.data ? <DetectedSetup detected={overview.data.detected} /> : undefined}
    >
      {overview.error ? (
        <Alert color='red'>{httpErrorToHuman(overview.error)}</Alert>
      ) : !overview.data ? (
        <Center py='xl'>
          <Spinner />
        </Center>
      ) : noProviders ? (
        <EmptyState
          icon={faPlug}
          title={tExt('pages.server.modpacks.blocked.title', {})}
          description={tExt('pages.server.modpacks.blocked.content', {})}
        />
      ) : (
        <>
          <InstallModpackModal
            opened={!!installTarget}
            onClose={() => setInstallTarget(null)}
            target={installTarget}
            overview={overview.data}
          />
          <Routes>
            <Route index element={<BrowseModpacksPage overview={overview.data} onInstall={setInstallTarget} />} />
            <Route
              path=':provider/:project/:tab?'
              element={<ModpackDetailsPage overview={overview.data} onInstall={setInstallTarget} />}
            />
          </Routes>
        </>
      )}
    </ServerContentContainer>
  );
}
