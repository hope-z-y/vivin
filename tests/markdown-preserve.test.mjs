import assert from 'node:assert/strict';
import test from 'node:test';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import remarkParse from 'remark-parse';
import remarkStringify from 'remark-stringify';
import { unified } from 'unified';
import { preserveMarkdownBlocks } from '../src/lib/markdown-preserve.ts';
import { equivalentMarkdown } from '../src/lib/markdown-roundtrip.ts';

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkMath)
  .use(remarkStringify, {
    handlers: { vivinSource: (node) => node.value },
  });

test('editable source blocks preserve frontmatter, nested HTML, references and footnotes while ordinary paragraphs remain rich text', () => {
  const markdown =
    '---\ntitle: Test\n---\n\n# Heading\n\nEditable **body**.\n\n<div>raw HTML</div>\n\n> nested <em>HTML</em>\n\n[link][ref] and note[^note].\n\n[ref]: https://example.com "title"\n\n[^note]: first\n\n    second paragraph\n';
  const tree = processor.parse(markdown);
  preserveMarkdownBlocks(tree, markdown);
  assert.equal(tree.children[1].type, 'heading');
  assert.equal(tree.children[2].type, 'paragraph');
  const sources = tree.children
    .filter((node) => node.type === 'vivinSource')
    .map((node) => node.value);
  assert.deepEqual(sources, [
    '---\ntitle: Test\n---',
    '<div>raw HTML</div>',
    '> nested <em>HTML</em>',
    '[link][ref] and note[^note].',
    '[ref]: https://example.com "title"',
    '[^note]: first\n\n    second paragraph',
  ]);
  assert.ok(equivalentMarkdown(markdown, processor.stringify(tree)));
  tree.children[2].children[0].value = 'Edited ';
  const saved = processor.stringify(tree);
  assert.ok(saved.includes('Edited **body**.'));
  for (const source of sources) assert.ok(saved.includes(source));
});

test('source preservation does not reinterpret fenced code or ordinary horizontal rules as frontmatter', () => {
  for (const markdown of [
    '# Title\n\n---\n\nText\n',
    '```html\n<script>source</script>\n```\n',
    '普通段落 **强调**。\n',
  ]) {
    const tree = processor.parse(markdown);
    const before = structuredClone(tree);
    preserveMarkdownBlocks(tree, markdown);
    assert.deepEqual(tree, before);
    assert.ok(equivalentMarkdown(markdown, processor.stringify(tree)));
  }
});
