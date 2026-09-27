import type { MantineColor, MantineSize } from '@mantine/core';
import Badge from '@/elements/data-display/Badge.tsx';
import type { Loader } from '../lib/schemas.ts';
import { useExtTranslations } from '../translations.ts';

export const loaderColors: Record<Loader, MantineColor> = {
  fabric: 'yellow',
  forge: 'blue',
  neoforge: 'orange',
  quilt: 'grape',
};

export default function LoaderBadge({ loader, size = 'sm' }: { loader: Loader; size?: MantineSize }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Badge color={loaderColors[loader]} variant='light' size={size}>
      {tExt(`loaders.${loader}`, {})}
    </Badge>
  );
}
