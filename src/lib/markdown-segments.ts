import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import remarkParse from 'remark-parse';
import { unified } from 'unified';

const parser = unified().use(remarkParse).use(remarkGfm).use(remarkMath);

export interface MarkdownSegment {
  id: number;
  content: string;
  serializedContent?: string;
  trailing: string;
  estimatedHeight: number;
}

export interface MarkdownSegmentRange {
  start: number;
  end: number;
}

export function markdownSegmentRanges(
  segments: readonly MarkdownSegment[],
): MarkdownSegmentRange[] {
  let start = 0;
  return segments.map((segment) => {
    const contentLength = (segment.serializedContent ?? segment.content).length;
    const end = start + contentLength + segment.trailing.length;
    const range = { start, end: start + contentLength };
    start = end;
    return range;
  });
}

function estimatedHeight(content: string): number {
  let height = 0;
  for (const line of content.split('\n')) {
    if (line.startsWith('```')) {
      height += 24;
    } else if (line.startsWith('|')) {
      height += 42;
    } else if (line.startsWith('#')) {
      height += 46;
    } else {
      height += Math.max(22, Math.ceil(line.length / 58) * 24);
    }
  }
  return Math.max(80, height);
}

export function splitMarkdown(markdown: string, targetLength = 3_500): MarkdownSegment[] {
  if (!markdown) return [{ id: 0, content: '', trailing: '', estimatedHeight: 80 }];
  const blocks = parser.parse(markdown).children;
  const boundaries = [0];
  let start = 0;
  for (const block of blocks) {
    const blockStart = block.position?.start.offset;
    const blockEnd = block.position?.end.offset;
    if (blockStart === undefined || blockEnd === undefined) continue;
    if (blockStart - start >= 500 && blockEnd - start > targetLength) {
      boundaries.push(blockStart);
      start = blockStart;
    }
  }
  boundaries.push(markdown.length);
  return boundaries.slice(0, -1).map((from, id) => {
    const raw = markdown.slice(from, boundaries[id + 1]);
    const trailing = raw.match(/\s*$/u)?.[0] ?? '';
    const content = raw.slice(0, raw.length - trailing.length);
    return { id, content, trailing, estimatedHeight: estimatedHeight(content) };
  });
}

export function joinMarkdown(segments: readonly MarkdownSegment[]): string {
  return segments
    .map((segment) => (segment.serializedContent ?? segment.content) + segment.trailing)
    .join('');
}
