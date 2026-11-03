import assert from 'node:assert/strict';
import test from 'node:test';
import { findDocumentMatches, replaceDocumentMatches } from '../src/lib/document-search.ts';
import { renderedRanges, renderedTextMap } from '../src/lib/rendered-text-map.ts';

test('file search uses literal, non-overlapping UTF-16 ranges and configurable word boundaries', () => {
  const source = '😀 Cat cat scatter cat_ 猫猫 [cat]';
  assert.deepEqual(findDocumentMatches(source, 'cat', { wholeWord: true }), [
    { offset: 3, length: 3 },
    { offset: 7, length: 3 },
    { offset: 28, length: 3 },
  ]);
  assert.equal(findDocumentMatches(source, 'cat', { caseSensitive: true }).length, 4);
  assert.deepEqual(findDocumentMatches('a.* a.*', '.*'), [
    { offset: 1, length: 2 },
    { offset: 5, length: 2 },
  ]);
  assert.equal(findDocumentMatches('aaa', 'aa').length, 1);
  assert.deepEqual(findDocumentMatches(source, ''), []);
});

test('single and all replacements preserve untouched Markdown and insert replacement literally', () => {
  const source = '---\ntitle: cat\n---\r\n\r\n**cat** and `cat` 😀\r\n';
  const matches = findDocumentMatches(source, 'cat');
  assert.equal(
    replaceDocumentMatches(source, [matches[1]], '$&猫'),
    source.replace('**cat**', () => '**$&猫**'),
  );
  assert.equal(
    replaceDocumentMatches(source, matches, ''),
    '---\ntitle: \n---\r\n\r\n**** and `` 😀\r\n',
  );
});

test('rendered anchors distinguish identical text in link destinations, code languages and body', () => {
  const source = '[repeat](repeat)\n\n```repeat\nrepeat\n```\n\nrepeat';
  const map = renderedTextMap(source, [
    { text: 'repeat', position: 1 },
    { text: 'repeat', position: 9 },
    { text: 'repeat', position: 17 },
  ]);
  const matches = findDocumentMatches(source, 'repeat');
  assert.deepEqual(
    matches.map((match) => renderedRanges(map, match.offset, match.length)),
    [[{ from: 1, to: 7 }], [], [], [{ from: 9, to: 15 }], [{ from: 17, to: 23 }]],
  );
});

test('highlight ranges cross formatting and block boundaries without changing Markdown', () => {
  const source = '**same** same\n\nsame';
  const map = renderedTextMap(source, [
    { text: 'same same', position: 1 },
    { text: 'same', position: 12 },
  ]);
  assert.deepEqual(renderedRanges(map, 0, source.length), [
    { from: 1, to: 10 },
    { from: 12, to: 16 },
  ]);
  assert.deepEqual(renderedRanges(map, source.lastIndexOf('same'), 4), [{ from: 12, to: 16 }]);
});

test('preserved source blocks retain distinct offsets including repeated raw content', () => {
  const source = '---\ntitle: raw\n---\n\n<div>raw</div>\n\n<div>raw</div>\n\nbody';
  const map = renderedTextMap(source, [
    { text: '---\ntitle: raw\n---', position: 1 },
    { text: '<div>raw</div>', position: 22 },
    { text: '<div>raw</div>', position: 39 },
    { text: 'body', position: 56 },
  ]);
  assert.equal(map.get(source.lastIndexOf('<div>')), 39);
  assert.equal(map.get(source.indexOf('body')), 56);
  const fallback = renderedTextMap(source, [{ text: source, position: 1 }]);
  assert.equal(fallback.get(source.indexOf('<div>')), source.indexOf('<div>') + 1);
});

test('escaped syntax and entities map to complete visible Unicode characters', () => {
  const source = '\\* &amp; &#x1f600; end';
  const map = renderedTextMap(source, [{ text: '* & 😀 end', position: 1 }]);
  assert.deepEqual(renderedRanges(map, source.indexOf('&amp;'), 5), [{ from: 3, to: 4 }]);
  assert.deepEqual(renderedRanges(map, source.indexOf('&#x'), 9), [{ from: 5, to: 7 }]);
  assert.deepEqual(renderedRanges(map, source.indexOf('end'), 3), [{ from: 8, to: 11 }]);
});

test('inline formula atoms keep following identical text anchored to the body', () => {
  const source = '$repeat$ repeat';
  const map = renderedTextMap(source, [
    { text: 'repeat', position: 1, atomic: true },
    { text: ' repeat', position: 2 },
  ]);
  assert.deepEqual(renderedRanges(map, 1, 6), [{ from: 1, to: 2 }]);
  assert.deepEqual(renderedRanges(map, source.lastIndexOf('repeat'), 6), [{ from: 3, to: 9 }]);
});
