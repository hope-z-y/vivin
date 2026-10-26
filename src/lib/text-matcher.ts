export interface TextMatch<T> {
  value: T;
  offset: number;
  length: number;
}

interface Node<T> {
  next: Map<string, number>;
  failure: number;
  outputs: { value: T; length: number }[];
}

/** Finds all literal patterns in one pass over a UTF-16 string. */
export function createTextMatcher<T>(patterns: { text: string; value: T }[]) {
  const nodes: Node<T>[] = [{ next: new Map(), failure: 0, outputs: [] }];
  for (const pattern of patterns) {
    if (!pattern.text) continue;
    let at = 0;
    for (const character of pattern.text) {
      let next = nodes[at]!.next.get(character);
      if (next === undefined) {
        next = nodes.length;
        nodes[at]!.next.set(character, next);
        nodes.push({ next: new Map(), failure: 0, outputs: [] });
      }
      at = next;
    }
    nodes[at]!.outputs.push({ value: pattern.value, length: pattern.text.length });
  }
  const queue: number[] = [];
  for (const child of nodes[0]!.next.values()) queue.push(child);
  for (let index = 0; index < queue.length; index++) {
    const parent = queue[index]!;
    for (const [character, child] of nodes[parent]!.next) {
      let failure = nodes[parent]!.failure;
      while (failure && !nodes[failure]!.next.has(character)) failure = nodes[failure]!.failure;
      nodes[child]!.failure = nodes[failure]!.next.get(character) ?? 0;
      nodes[child]!.outputs.push(...nodes[nodes[child]!.failure]!.outputs);
      queue.push(child);
    }
  }
  return (text: string): TextMatch<T>[] => {
    const matches: TextMatch<T>[] = [];
    let at = 0;
    let offset = 0;
    for (const character of text) {
      while (at && !nodes[at]!.next.has(character)) at = nodes[at]!.failure;
      at = nodes[at]!.next.get(character) ?? 0;
      offset += character.length;
      for (const output of nodes[at]!.outputs)
        matches.push({
          value: output.value,
          offset: offset - output.length,
          length: output.length,
        });
    }
    return matches;
  };
}
