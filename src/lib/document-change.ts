export function minimalDocumentChange(previous: string, next: string) {
  let from = 0;
  while (from < previous.length && from < next.length && previous[from] === next[from]) from++;
  let to = previous.length;
  let nextEnd = next.length;
  while (to > from && nextEnd > from && previous[to - 1] === next[nextEnd - 1]) {
    to--;
    nextEnd--;
  }
  return { from, to, insert: next.slice(from, nextEnd) };
}
