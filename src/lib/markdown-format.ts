import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import remarkParse from 'remark-parse';
import remarkStringify from 'remark-stringify';
import { unified } from 'unified';
import { equivalentMarkdown } from './markdown-roundtrip.ts';

const formatter = unified().use(remarkParse).use(remarkGfm).use(remarkMath).use(remarkStringify, {
  bullet: '-',
  emphasis: '*',
  strong: '*',
  fences: true,
  listItemIndent: 'one',
  rule: '-',
});

export function formatMarkdown(markdown: string): string {
  if (!markdown.trim()) return markdown;
  // Frontmatter is opaque project content, not a heading to normalize.
  const prefix =
    markdown.match(/^(?:\uFEFF)?---\r?\n[\s\S]*?\r?\n(?:---|\.\.\.)(?:\r?\n|$)/)?.[0] ?? '';
  const body = markdown.slice(prefix.length);
  let formatted = prefix + String(formatter.processSync(body));
  if (markdown.includes('\r\n')) formatted = formatted.replace(/\r?\n/g, '\r\n');
  if (!equivalentMarkdown(markdown, formatted)) {
    throw new Error('Markdown formatting would change document structure');
  }
  return formatted;
}
