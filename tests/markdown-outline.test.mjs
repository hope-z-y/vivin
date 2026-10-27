import assert from 'node:assert/strict';
import test from 'node:test';
import { markdownHeadings } from '../src/lib/markdown-outline.ts';

test('outline includes ATX and setext headings with source offsets', () => {
  const markdown = [
    '# 总览',
    '',
    '## **审批**与 `流程`',
    '',
    '重复标题',
    '------',
    '',
    '重复标题',
    '------',
  ].join('\n');
  const headings = markdownHeadings(markdown);

  assert.deepEqual(
    headings.map(({ depth, text }) => [depth, text]),
    [
      [1, '总览'],
      [2, '审批与 流程'],
      [2, '重复标题'],
      [2, '重复标题'],
    ],
  );
  assert.deepEqual(
    headings.map(({ offset }) => markdown.slice(offset).split('\n')[0]),
    ['# 总览', '## **审批**与 `流程`', '重复标题', '重复标题'],
  );
});

test('outline ignores heading syntax inside code blocks and blockquotes', () => {
  const markdown = ['```md', '# 不是标题', '```', '', '> # 引用标题', '', '### 正文'].join('\n');
  assert.deepEqual(
    markdownHeadings(markdown).map(({ text }) => text),
    ['正文'],
  );
});
