import assert from 'node:assert/strict';
import test from 'node:test';
import {
  joinMarkdown,
  markdownSegmentRanges,
  splitMarkdown,
} from '../src/lib/markdown-segments.ts';

test('segments preserve Markdown bytes and keep compound blocks intact', () => {
  const source = [
    '# 标题\r',
    '',
    ...Array.from({ length: 12 }, (_, index) => `段落 ${index} ${'说明'.repeat(20)}\r`),
    '',
    '| 项目 | 内容 |\r',
    '| --- | --- |\r',
    '| A | ' + '表格'.repeat(80) + ' |\r',
    '',
    '```mermaid\r',
    'flowchart LR\r',
    '  A --> B\r',
    '```\r',
    '',
    '- 一级\r',
    '  - 二级\r',
    '',
  ].join('\n');
  const segments = splitMarkdown(source, 200);

  assert.ok(segments.length > 1);
  assert.equal(joinMarkdown(segments), source);
  assert.ok(segments.some((segment) => segment.content.includes('| A |')));
  assert.ok(segments.some((segment) => segment.content.includes('```mermaid')));
  assert.ok(segments.every((segment) => !segment.content.startsWith('  A --> B')));
});

test('edited segment can keep editor whitespace without duplicating document separators', () => {
  const source = `# A\n\n${'a'.repeat(600)}\n\n# B\n`;
  const segments = splitMarkdown(source, 5);
  assert.equal(segments.length, 2);
  segments[0] = {
    ...segments[0],
    content: segments[0].content + '\n',
    serializedContent: segments[0].content,
  };
  assert.equal(joinMarkdown(segments), source);
  assert.notEqual(
    markdownSegmentRanges(segments)[1].start,
    segments[0].content.length + segments[0].trailing.length,
  );
  const edited = joinMarkdown(segments).indexOf('# B');
  assert.equal(markdownSegmentRanges(segments)[1].start, edited);
});
