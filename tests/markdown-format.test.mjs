import assert from 'node:assert/strict';
import test from 'node:test';
import { formatMarkdown } from '../src/lib/markdown-format.ts';
import { minimalDocumentChange } from '../src/lib/document-change.ts';
import { equivalentMarkdown } from '../src/lib/markdown-roundtrip.ts';

test('formatting normalizes headings, lists and tables without changing Markdown meaning', () => {
  const source = '#   标题\n\n*  一项\n*  二项\n\n|a|b|\n|-|-|\n|1|2|\n';
  const result = formatMarkdown(source);
  assert.ok(result.startsWith('# 标题\n\n- 一项\n- 二项\n'));
  assert.ok(result.includes('| a | b |'));
  assert.ok(equivalentMarkdown(source, result));
  assert.equal(formatMarkdown(result), result);
});

test('formatting retains frontmatter, code, hard breaks, formulas, HTML and references', () => {
  const prefix = '---\ntitle:  原样保留\ncustom: [a,  b]\n---\n';
  const source =
    prefix +
    '\n#   标题\n\n第一行  \n第二行\n\n```mermaid\nflowchart LR\n  A[中文] --> B[结束]\n```\n\n$$\nx^2 + y^2\n$$\n\n<div data-custom="x">原始 HTML</div>\n\n[引用][ref]\n\n[ref]: other.md "标题"\n\n[^note]: 脚注内容\n';
  const result = formatMarkdown(source);
  assert.ok(result.startsWith(prefix));
  assert.ok(result.includes('flowchart LR\n  A[中文] --> B[结束]'));
  assert.ok(result.includes('<div data-custom="x">原始 HTML</div>'));
  assert.ok(equivalentMarkdown(source, result));
  assert.equal(formatMarkdown(result), result);
});

test('formatting preserves CRLF and leaves empty documents untouched', () => {
  const source = '#   标题\r\n\r\n一段文字\r\n';
  const result = formatMarkdown(source);
  assert.ok(result.includes('\r\n'));
  assert.ok(!result.replaceAll('\r\n', '').includes('\n'));
  assert.equal(formatMarkdown(''), '');
  assert.equal(formatMarkdown(' \n'), ' \n');
});

test('external document replacement only changes the differing range for cursor mapping and undo', () => {
  const previous = '#   标题\n\n正文不变';
  const next = '# 标题\n\n正文不变';
  const change = minimalDocumentChange(previous, next);
  assert.equal(previous.slice(0, change.from) + change.insert + previous.slice(change.to), next);
  assert.equal(change.to - change.from, 2);
  assert.equal(change.insert, '');
});
