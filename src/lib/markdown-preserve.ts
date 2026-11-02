export interface MarkdownNode {
  type: string;
  children?: MarkdownNode[];
  value?: string;
  position?: { start: { offset?: number }; end: { offset?: number } };
}

const rawTypes = new Set([
  'html',
  'definition',
  'linkReference',
  'imageReference',
  'footnoteDefinition',
  'footnoteReference',
]);

function needsSource(node: MarkdownNode): boolean {
  return rawTypes.has(node.type) || (node.children?.some(needsSource) ?? false);
}

// Keep syntax outside Crepe's schema as editable source blocks in the same
// document. In particular, never silently drop HTML, references or footnotes.
export function preserveMarkdownBlocks(tree: MarkdownNode, source: string) {
  if (!tree.children) return;
  const frontmatterEnd =
    source.match(/^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/)?.[0].trimEnd().length ?? 0;
  const children: MarkdownNode[] = [];
  let frontmatterAdded = false;
  for (const node of tree.children) {
    const from = node.position?.start.offset;
    const to = node.position?.end.offset;
    if (from === undefined || to === undefined) {
      children.push(node);
      continue;
    }
    if (from < frontmatterEnd) {
      if (!frontmatterAdded) {
        children.push({ type: 'vivinSource', value: source.slice(0, frontmatterEnd) });
        frontmatterAdded = true;
      }
      continue;
    }
    children.push(
      needsSource(node) ? { type: 'vivinSource', value: source.slice(from, to) } : node,
    );
  }
  tree.children = children;
}
