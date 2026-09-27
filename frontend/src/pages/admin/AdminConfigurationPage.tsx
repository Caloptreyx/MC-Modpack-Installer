import { faCheck, faTrash, faXmark } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center, Group, Stack } from '@mantine/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import TitleCard from '@/elements/data-display/TitleCard.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import PasswordInput from '@/elements/input/PasswordInput.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TagsInput from '@/elements/input/TagsInput.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import getSettings from '../../api/admin/getSettings.ts';
import updateSettings from '../../api/admin/updateSettings.ts';
import verifyCurseforgeKey from '../../api/admin/verifyCurseforgeKey.ts';
import { ProviderDot } from '../../components/ProviderMark.tsx';
import { modpackQueryKeys } from '../../lib/queryKeys.ts';
import type { AdminSettings } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';

export default function AdminConfigurationPage() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const canManage = useAdminCan('modpacks.manage');

  const { data, isLoading, error } = useQuery({
    queryKey: modpackQueryKeys.adminSettings(),
    queryFn: getSettings,
  });

  const [settings, setSettings] = useState<AdminSettings | null>(null);
  // undefined keeps the stored key, '' removes it
  const [apiKey, setApiKey] = useState<string | undefined>(undefined);
  const [saving, setSaving] = useState(false);
  const [verifying, setVerifying] = useState(false);

  useEffect(() => {
    if (data) setSettings(data);
  }, [data]);

  if (isLoading) {
    return (
      <Center py='lg'>
        <Spinner />
      </Center>
    );
  }
  if (error || !settings) {
    return <Alert color='red'>{error ? httpErrorToHuman(error) : null}</Alert>;
  }

  const update = (patch: Partial<AdminSettings>) => setSettings({ ...settings, ...patch });
  const keyRemoved = apiKey === '';
  const keyConfigured = settings.curseforgeConfigured && !keyRemoved;

  const doSave = async () => {
    setSaving(true);
    try {
      const saved = await updateSettings({
        modrinthEnabled: settings.modrinthEnabled,
        curseforgeEnabled: settings.curseforgeEnabled,
        curseforgeApiKey: apiKey,
        allowCleanInstall: settings.allowCleanInstall,
        installerImage: settings.installerImage,
        excludedMods: settings.excludedMods,
      });
      setSettings(saved);
      setApiKey(undefined);
      queryClient.setQueryData(modpackQueryKeys.adminSettings(), saved);
      addToast(tExt('pages.admin.modpacks.toast.saved', {}), 'success');
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    } finally {
      setSaving(false);
    }
  };

  const doVerify = async () => {
    setVerifying(true);
    try {
      await verifyCurseforgeKey(apiKey || undefined);
      addToast(tExt('pages.admin.modpacks.providers.verified', {}), 'success');
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    } finally {
      setVerifying(false);
    }
  };

  return (
    <Stack gap='lg'>
      <Tabs defaultValue='providers'>
        <Tabs.List>
          <Tabs.Tab value='providers'>{tExt('pages.admin.modpacks.tabs.providers', {})}</Tabs.Tab>
          <Tabs.Tab value='installation'>{tExt('pages.admin.modpacks.tabs.installation', {})}</Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value='providers' pt='md'>
          <Stack gap='md'>
            <TitleCard title={tExt('providers.modrinth', {})} icon={<ProviderDot provider='modrinth' />}>
              <Switch
                label={tExt('pages.admin.modpacks.providers.modrinth', {})}
                description={tExt('pages.admin.modpacks.providers.modrinthDescription', {})}
                checked={settings.modrinthEnabled}
                disabled={!canManage}
                onChange={(event) => update({ modrinthEnabled: event.currentTarget.checked })}
              />
            </TitleCard>

            <TitleCard
              title={tExt('providers.curseforge', {})}
              icon={<ProviderDot provider='curseforge' />}
              rightSection={
                <Badge
                  color={keyConfigured ? 'green' : 'gray'}
                  variant='light'
                  leftSection={<FontAwesomeIcon icon={keyConfigured ? faCheck : faXmark} />}
                >
                  {keyConfigured
                    ? tExt('pages.admin.modpacks.providers.status.configured', {})
                    : tExt('pages.admin.modpacks.providers.status.missing', {})}
                </Badge>
              }
            >
              <Stack gap='md'>
                <Switch
                  label={tExt('pages.admin.modpacks.providers.curseforge', {})}
                  description={tExt('pages.admin.modpacks.providers.curseforgeDescription', {})}
                  checked={settings.curseforgeEnabled}
                  disabled={!canManage}
                  onChange={(event) => update({ curseforgeEnabled: event.currentTarget.checked })}
                />
                <PasswordInput
                  label={tExt('pages.admin.modpacks.providers.apiKey', {})}
                  description={
                    <Text size='xs' c='dimmed' component='span'>
                      {tExt('pages.admin.modpacks.providers.apiKeyDescription', {}).md()}
                      {keyConfigured && ` ${tExt('pages.admin.modpacks.providers.apiKeyConfigured', {})}`}
                    </Text>
                  }
                  placeholder={tExt('pages.admin.modpacks.providers.apiKeyPlaceholder', {})}
                  value={apiKey ?? ''}
                  disabled={!canManage}
                  onChange={(event) => setApiKey(event.target.value || undefined)}
                />
                <Group gap='sm'>
                  <Button
                    variant='default'
                    loading={verifying}
                    disabled={!canManage || (!apiKey && !keyConfigured)}
                    onClick={doVerify}
                  >
                    {tExt('pages.admin.modpacks.providers.verify', {})}
                  </Button>
                  {settings.curseforgeConfigured && (
                    <Button
                      variant='subtle'
                      color='red'
                      leftSection={<FontAwesomeIcon icon={faTrash} />}
                      disabled={!canManage || keyRemoved}
                      onClick={() => setApiKey('')}
                    >
                      {tExt('pages.admin.modpacks.providers.remove', {})}
                    </Button>
                  )}
                </Group>
              </Stack>
            </TitleCard>
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value='installation' pt='md'>
          <Stack gap='md'>
            <Switch
              label={tExt('pages.admin.modpacks.installation.allowCleanInstall', {})}
              description={tExt('pages.admin.modpacks.installation.allowCleanInstallDescription', {})}
              checked={settings.allowCleanInstall}
              disabled={!canManage}
              onChange={(event) => update({ allowCleanInstall: event.currentTarget.checked })}
            />
            <TextInput
              withAsterisk
              label={tExt('pages.admin.modpacks.installation.installerImage', {})}
              description={tExt('pages.admin.modpacks.installation.installerImageDescription', {})}
              value={settings.installerImage}
              disabled={!canManage}
              onChange={(event) => update({ installerImage: event.target.value })}
            />
            <TagsInput
              label={tExt('pages.admin.modpacks.installation.excludedMods', {})}
              description={tExt('pages.admin.modpacks.installation.excludedModsDescription', {})}
              value={settings.excludedMods}
              onChange={(excludedMods) => update({ excludedMods })}
            />
          </Stack>
        </Tabs.Panel>
      </Tabs>

      <Group justify='flex-end'>
        <AdminCan action='modpacks.manage' cantSave>
          <Button onClick={doSave} loading={saving} disabled={!settings.installerImage.trim()}>
            {tExt('pages.admin.modpacks.button.save', {})}
          </Button>
        </AdminCan>
      </Group>
    </Stack>
  );
}
