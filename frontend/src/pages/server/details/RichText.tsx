import Markdown from 'react-markdown';
import rehypeRaw from 'rehype-raw';
import rehypeSanitize from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';
import { markdownComponents } from '@/providers/TranslationProvider.tsx';
import type { TextFormat } from '../../../lib/schemas.ts';

interface Props {
  content: string;
  format: TextFormat;
}

/**
 * Renders Modrinth markdown or CurseForge HTML through a sanitizing
 * markdown renderer. Indentation is stripped from HTML so nested tags are not
 * mistaken for markdown code blocks.
 */
export default function RichText({ content, format }: Props) {
  const source = format === 'html' ? content.replace(/^[ \t]+/gm, '') : content;

  return (
    <div className='mpi-rich-text'>
      <Markdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeRaw, rehypeSanitize]} components={markdownComponents}>
        {source}
      </Markdown>
    </div>
  );
}
