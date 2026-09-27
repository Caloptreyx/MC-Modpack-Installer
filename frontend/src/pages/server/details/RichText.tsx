import type { TextFormat } from '../../../lib/schemas.ts';

interface Props {
  content: string;
  format: TextFormat;
}

/**
 * Renders Modrinth markdown or CurseForge HTML through the panel's sanitizing
 * markdown renderer. Indentation is stripped from HTML so nested tags are not
 * mistaken for markdown code blocks.
 */
export default function RichText({ content, format }: Props) {
  const source = format === 'html' ? content.replace(/^[ \t]+/gm, '') : content;

  return <div className='mpi-rich-text'>{source.md({ html: true })}</div>;
}
