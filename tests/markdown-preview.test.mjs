import assert from 'node:assert/strict';
import test from 'node:test';
import { previewSections } from '../src/lib/markdown-preview.ts';
import { mermaidSource } from '../src/lib/mermaid-source.ts';

function allNodes(sections) {
  const result = [];
  function visit(node) {
    result.push(node);
    for (const child of node.children ?? []) visit(child);
  }
  for (const section of sections) for (const node of section.nodes) visit(node);
  return result;
}

test('long preview resolves global references and footnotes across virtual sections', () => {
  const source =
    '# 总览\n\n[跨段引用][target]，说明[^note]。\n\n' +
    Array.from(
      { length: 300 },
      (_, i) => `## 章节 ${i}\n\n${'这是一段长文内容。'.repeat(40)}\n\n`,
    ).join('') +
    '[target]: other.md\n\n[^note]: 文末脚注\n';
  assert.ok(source.length > 80_000);
  const sections = previewSections(source);
  assert.ok(sections.length > 10);
  const nodes = allNodes(sections);
  assert.ok(nodes.some((node) => node.tagName === 'a' && node.properties.href === 'other.md'));
  for (const link of nodes.filter(
    (node) => node.tagName === 'a' && String(node.properties.href).startsWith('#'),
  )) {
    assert.ok(
      nodes.some((node) => node.properties?.id === link.properties.href.slice(1)),
      link.properties.href,
    );
  }
  assert.equal(
    nodes.filter(
      (node) => node.tagName === 'h2' && node.properties['data-source-offset'] !== undefined,
    ).length,
    300,
  );
  const heading = nodes.find((node) => node.tagName === 'h2');
  assert.ok(source.slice(heading.properties['data-source-offset']).startsWith('## 章节 0'));
});

test('preview escapes raw HTML and rejects unsafe links without losing source text', () => {
  const source =
    '# <img src=x onerror=alert(1)>\n\n<script>alert(1)</script>\n\n[危险](javascript:alert%281%29)\n\n![外部](https://example.com/image.png)\n';
  const nodes = allNodes(previewSections(source));
  assert.ok(!nodes.some((node) => node.tagName === 'script'));
  assert.ok(!nodes.some((node) => node.properties?.onError || node.properties?.onerror));
  assert.ok(!nodes.some((node) => String(node.properties?.href).startsWith('javascript:')));
  assert.ok(nodes.some((node) => node.type === 'text' && node.value.includes('<script>')));
});

test('preview supports GFM, formulas, Mermaid, and unsupported HTML without an editable roundtrip', () => {
  const source =
    '- [x] 完成\n\n| 项目 | 数值 |\n| --- | ---: |\n| A | 1 |\n\n$x^2$\n\n$$\nx^2 + y^2\n$$\n\n```mermaid\nflowchart LR\n A --> B\n```\n\n<div>保留内容</div>\n';
  const nodes = allNodes(previewSections(source, 10));
  assert.ok(nodes.some((node) => node.tagName === 'table'));
  assert.ok(
    nodes.some(
      (node) =>
        node.tagName === 'input' &&
        node.properties.checked === true &&
        node.properties.disabled === true,
    ),
  );
  assert.ok(nodes.some((node) => node.properties?.className?.includes('katex')));
  assert.ok(
    nodes.some(
      (node) => node.type === 'element' && mermaidSource(node) === 'flowchart LR\n A --> B',
    ),
  );
  assert.ok(nodes.some((node) => node.type === 'text' && node.value === '<div>保留内容</div>'));
});

test('preview decodes entities and keeps untrusted math commands from creating links', () => {
  const nodes = allNodes(
    previewSections('字符 &amp; &copy; &#x4e2d;\n\n$\\href{javascript:alert(1)}{click}$\n'),
  );
  assert.ok(nodes.some((node) => node.type === 'text' && node.value.includes('字符 & © 中')));
  assert.ok(!nodes.some((node) => node.tagName === 'a'));
});
