import { faCubes } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Avatar } from '@mantine/core';

export default function ModpackIcon({ url, name, size = 64 }: { url: string | null; name: string; size?: number }) {
  return (
    <Avatar src={url} alt={name} size={size} radius='md' color='gray' variant='light'>
      <FontAwesomeIcon icon={faCubes} size={size >= 64 ? 'xl' : 'lg'} />
    </Avatar>
  );
}
