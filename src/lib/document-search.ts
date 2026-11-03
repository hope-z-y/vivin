export interface SearchRange {
  offset: number;
  length: number;
}

export interface DocumentSearchOptions {
  caseSensitive?: boolean;
  wholeWord?: boolean;
}

export function findDocumentMatches(
  content: string,
  query: string,
  options: DocumentSearchOptions = {},
): SearchRange[] {
  if (!query) return [];
  const pattern = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const expression = new RegExp(pattern, options.caseSensitive ? 'gu' : 'giu');
  const word = /[\p{L}\p{N}_]/u;
  return Array.from(content.matchAll(expression)).flatMap((match) => {
    const offset = match.index;
    const length = match[0].length;
    const beforeCharacters = Array.from(content.slice(Math.max(0, offset - 2), offset));
    const before = beforeCharacters[beforeCharacters.length - 1] ?? '';
    const after = Array.from(content.slice(offset + length, offset + length + 2))[0] ?? '';
    return options.wholeWord && (word.test(before) || word.test(after)) ? [] : [{ offset, length }];
  });
}

export function replaceDocumentMatches(
  content: string,
  matches: SearchRange[],
  replacement: string,
): string {
  let end = 0;
  const chunks: string[] = [];
  for (const match of matches) {
    chunks.push(content.slice(end, match.offset), replacement);
    end = match.offset + match.length;
  }
  chunks.push(content.slice(end));
  return chunks.join('');
}
