import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import remarkParse from 'remark-parse';
import { unified } from 'unified';

const parser = unified().use(remarkParse).use(remarkGfm).use(remarkMath);

export interface MarkdownHeading {
  depth: number;
  offset: number;
  text: string;
}

function visibleText(node: unknown): string {
  if (!node || typeof node !== 'object') return '';
  if ('value' in node && typeof node.value === 'string') return node.value;
  if ('alt' in node && typeof node.alt === 'string') return node.alt;
  if ('children' in node && Array.isArray(node.children))
    return node.children.map(visibleText).join('');
  return '';
}

export function markdownHeadings(markdown: string): MarkdownHeading[] {
  const headings: MarkdownHeading[] = [];
  for (const node of parser.parse(markdown).children) {
    if (node.type !== 'heading') continue;
    const offset = node.position?.start.offset;
    if (offset === undefined) continue;
    headings.push({ depth: node.depth, offset, text: visibleText(node).trim() });
  }
  return headings;
}
