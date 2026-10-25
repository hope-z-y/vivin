import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../examples/vivin-feature-tour',
);
const metadataDirectory = path.join(root, '.vivin');
const checkOnly = process.argv.includes('--check');
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8');

function anchor(document, quote, occurrence = 0) {
  const content = read(document);
  let index = -1;
  let from = 0;
  for (let seen = 0; seen <= occurrence; seen++) {
    index = content.indexOf(quote, from);
    if (index < 0) throw new Error(`找不到演示短语：${document} / ${quote} / ${occurrence}`);
    from = index + quote.length;
  }
  return {
    // JavaScript string length matches the UTF-16 offsets used by the app.
    offset: content.slice(0, index).length,
    contextBefore: content.slice(Math.max(0, index - 32), index),
    contextAfter: content.slice(index + quote.length, index + quote.length + 32),
  };
}

const annotationDocument = '标注与术语.md';
const annotations = [
  [
    '10000000-0000-4000-8000-000000000001',
    'comment',
    '术语解释应在项目内复用',
    '这条解释应在所有同名术语的文档中共用。',
  ],
  [
    '10000000-0000-4000-8000-000000000002',
    'todo',
    '确认发布窗口',
    '发布前确认负责人、日期和版本。',
  ],
  [
    '10000000-0000-4000-8000-000000000003',
    'question',
    '这条规则是否需要例外',
    '请确认是否允许某些文档跳过这条规则。',
  ],
  [
    '10000000-0000-4000-8000-000000000004',
    'error',
    '示例金额写成了 1000 元',
    '演示用错误：正确金额应为 100 元。',
  ],
].map(([id, kind, quote, note]) => ({
  id,
  document: annotationDocument,
  kind,
  quote,
  ...anchor(annotationDocument, quote),
  note,
}));
annotations.push({
  id: '10000000-0000-4000-8000-000000000005',
  document: annotationDocument,
  kind: 'comment',
  quote: '已被删掉的演示文本',
  offset: 0,
  contextBefore: '',
  contextAfter: '',
  note: '演示无法准确定位的标注：只在项目标注列表中出现，不应出现在错误的正文位置。',
});

const termId = '20000000-0000-4000-8000-000000000002';
const projectData = {
  schemaVersion: 1,
  pinned: ['00-从这里开始.md', '项目管理/固定在前面的文档.md'],
  locked: ['项目管理/直接锁定.md', '项目管理/继承锁定'],
  terms: [
    {
      id: '20000000-0000-4000-8000-000000000001',
      name: '项目快照',
      definition: '复制整个项目文件夹后仍保留的文档、资源和结构化数据。',
    },
    {
      id: termId,
      name: '上下文锚点',
      definition: '使用选中文本和前后语境，在文档修改后重新定位标注。',
    },
    {
      id: '20000000-0000-4000-8000-000000000003',
      name: '位置缩略图',
      definition: '文档右侧按大致位置显示各种标注的轻量标记栏。',
    },
  ],
  termExceptions: [
    {
      id: '30000000-0000-4000-8000-000000000001',
      document: annotationDocument,
      termId,
      ...anchor(annotationDocument, '上下文锚点', 2),
    },
  ],
  annotations,
};

const historyDocument = 'AI协作/AI修改记录示例.md';
const afterContent = read(historyDocument);
const beforeContent = afterContent.replace('第一轮验收已经完成', '第一轮验收仍在进行');
if (beforeContent === afterContent) throw new Error('演示修改记录没有正文变化');
const history = [
  {
    id: '40000000-0000-4000-8000-000000000001',
    time: Math.floor(Date.UTC(2026, 9, 24, 10, 0, 0) / 1000),
    operation: 'write_document',
    target: historyDocument,
    beforeContent,
    afterContent,
    beforeData: null,
    afterData: null,
  },
];

fs.mkdirSync(metadataDirectory, { recursive: true });
for (const [name, value] of [
  ['project.json', projectData],
  ['history.json', history],
]) {
  const file = path.join(metadataDirectory, name);
  const expected = `${JSON.stringify(value, null, 2)}\n`;
  if (checkOnly) {
    if (JSON.stringify(JSON.parse(fs.readFileSync(file, 'utf8'))) !== JSON.stringify(value)) {
      throw new Error(`${name} 与演示文档不一致；运行 node scripts/generate-feature-tour-data.mjs`);
    }
  } else {
    fs.writeFileSync(file, expected);
  }
}

console.log(checkOnly ? '演示项目数据校验通过' : '演示项目数据已生成');
