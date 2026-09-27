import { faImage } from '@fortawesome/free-solid-svg-icons';
import { Image, SimpleGrid, Stack, UnstyledButton } from '@mantine/core';
import { useState } from 'react';
import Card from '@/elements/data-display/Card.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import { Modal } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import type { ModpackDetails } from '../../../lib/schemas.ts';
import { useExtTranslations } from '../../../translations.ts';

type GalleryImage = ModpackDetails['gallery'][number];

export default function GalleryTab({ gallery }: { gallery: GalleryImage[] }) {
  const { t: tExt } = useExtTranslations();
  const [opened, setOpened] = useState<GalleryImage | null>(null);

  if (gallery.length === 0) {
    return <EmptyState icon={faImage} title={tExt('pages.server.modpacks.gallery.empty', {})} description='' />;
  }

  return (
    <>
      <Modal opened={!!opened} onClose={() => setOpened(null)} size='70rem' title={opened?.title}>
        {opened && (
          <Stack gap='sm'>
            <Image src={opened.url} alt={opened.title ?? ''} radius='md' fit='contain' mah='75vh' />
            {opened.description && <Text c='dimmed'>{opened.description}</Text>}
          </Stack>
        )}
      </Modal>

      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }}>
        {gallery.map((image) => (
          <Card key={image.url} p={0} hoverable style={{ overflow: 'hidden' }}>
            <UnstyledButton onClick={() => setOpened(image)} w='100%'>
              <Image src={image.url} alt={image.title ?? ''} h={200} fit='cover' />
              {(image.title || image.description) && (
                <Stack gap={2} p='sm'>
                  {image.title && (
                    <Text fw={500} size='sm' truncate>
                      {image.title}
                    </Text>
                  )}
                  {image.description && (
                    <Text size='xs' c='dimmed' lineClamp={2}>
                      {image.description}
                    </Text>
                  )}
                </Stack>
              )}
            </UnstyledButton>
          </Card>
        ))}
      </SimpleGrid>
    </>
  );
}
