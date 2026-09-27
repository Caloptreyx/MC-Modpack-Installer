import type { MantineColor } from '@mantine/core';
import Badge from '@/elements/data-display/Badge.tsx';
import type { ReleaseType } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

const colors: Record<ReleaseType, MantineColor> = {
  release: 'green',
  beta: 'yellow',
  alpha: 'red',
};

export default function ReleaseTypeBadge({ releaseType }: { releaseType: ReleaseType }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Badge color={colors[releaseType]} variant='light' size='sm'>
      {tExt(`releaseTypes.${releaseType}`, {})}
    </Badge>
  );
}
