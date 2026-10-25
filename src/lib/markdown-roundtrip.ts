import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import remarkParse from 'remark-parse';
import { unified } from 'unified';

const parser = unified().use(remarkParse).use(remarkGfm).use(remarkMath);

function withoutPositions(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(withoutPositions);
  if (value && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => key !== 'position')
        .map(([key, item]) => [key, withoutPositions(item)]),
    );
  return value;
}

export function equivalentMarkdown(left: string, right: string): boolean {
  if (left.trimEnd() === right.trimEnd()) return true;
  try {
    return (
      JSON.stringify(withoutPositions(parser.parse(left))) ===
      JSON.stringify(withoutPositions(parser.parse(right)))
    );
  } catch {
    return false;
  }
}
