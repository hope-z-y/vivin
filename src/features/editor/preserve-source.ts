import { InitReady, remarkPluginsCtx } from '@milkdown/kit/core';
import type { MilkdownPlugin } from '@milkdown/kit/ctx';
import type { RemarkPlugin } from '@milkdown/kit/transformer';
import { $node } from '@milkdown/kit/utils';
import { preserveMarkdownBlocks } from '../../lib/markdown-preserve';

export const preserveSource: MilkdownPlugin = (context) => async () => {
  await context.wait(InitReady);
  const plugin: RemarkPlugin = {
    plugin: () => (tree, file) => {
      preserveMarkdownBlocks(tree, String(file.value));
    },
    options: {},
  };
  // Run before the preset expands references or wraps HTML; offsets still
  // describe the original Markdown here.
  context.update(remarkPluginsCtx, (plugins) => [plugin, ...plugins]);
  return () => {
    context.update(remarkPluginsCtx, (plugins) => plugins.filter((item) => item !== plugin));
  };
};

export const sourceBlock = $node('vivin_source', () => ({
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,
  defining: true,
  parseDOM: [{ tag: 'pre[data-vivin-source]', preserveWhitespace: 'full' }],
  toDOM: () => ['pre', { 'data-vivin-source': '' }, ['code', 0]],
  parseMarkdown: {
    match: (node) => node.type === 'vivinSource',
    runner: (state, node, type) => {
      state.openNode(type);
      if (typeof node.value === 'string') state.addText(node.value);
      state.closeNode();
    },
  },
  toMarkdown: {
    match: (node) => node.type.name === 'vivin_source',
    runner: (state, node) => {
      state.addNode('html', undefined, node.textContent);
    },
  },
}));
