import { Center } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { httpErrorToHuman } from '@/api/axios.ts';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import { Modal } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import getVersion from '../../../api/server/getVersion.ts';
import { modpackQueryKeys } from '../../../lib/queryKeys.ts';
import type { ModpackVersion, Provider } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';
import RichText from './RichText.tsx';

interface Props {
  provider: Provider;
  projectId: string;
  version: ModpackVersion | null;
  onClose: () => void;
}

export default function ChangelogModal({ provider, projectId, version, onClose }: Props) {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();

  const details = useQuery({
    queryKey: modpackQueryKeys.version(server.uuid, provider, projectId, version?.id ?? ''),
    queryFn: () => getVersion(server.uuid, provider, projectId, version!.id),
    enabled: !!version,
  });

  return (
    <Modal
      opened={!!version}
      onClose={onClose}
      size='xl'
      title={version ? tExt('pages.server.modpacks.versions.changelogModal.title', { version: version.name }) : null}
    >
      {details.isLoading ? (
        <Center py='lg'>
          <Spinner />
        </Center>
      ) : details.error ? (
        <Alert color='red'>{httpErrorToHuman(details.error)}</Alert>
      ) : details.data?.changelog.trim() ? (
        <RichText content={details.data.changelog} format={details.data.changelogFormat} />
      ) : (
        <Text c='dimmed'>{tExt('pages.server.modpacks.versions.changelogModal.empty', {})}</Text>
      )}
    </Modal>
  );
}
