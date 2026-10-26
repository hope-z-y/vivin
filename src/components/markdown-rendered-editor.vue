<script setup lang="ts">
import { Crepe, CrepeFeature } from '@milkdown/crepe';
import { editorViewCtx } from '@milkdown/kit/core';
import type { Ctx } from '@milkdown/kit/ctx';
import { uploadConfig } from '@milkdown/kit/plugin/upload';
import { Plugin, PluginKey, type EditorState } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { $prose, replaceAll } from '@milkdown/kit/utils';
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { AnnotationMark } from '../lib/annotation-anchor';
import type { AnnotationKind } from '../lib/project-api';
import { equivalentMarkdown } from '../lib/markdown-roundtrip';
import { createTextMatcher } from '../lib/text-matcher';
import { getCachedMermaidPreview, renderMermaidPreview } from '../lib/mermaid-preview';
import { importImage, readProjectImage } from '../lib/project-api';
import LoadingState from './loading-state.vue';
import '@milkdown/crepe/theme/common/style.css';
import '@milkdown/crepe/theme/frame.css';

const props = defineProps<{
  modelValue: string;
  documentPath: string;
  readonly?: boolean;
  annotations?: AnnotationMark[];
  terms?: { id: string; offset: number; quote: string }[];
  locale?: 'zh' | 'en';
}>();
const emit = defineEmits<{
  'update:modelValue': [value: string];
  incompatible: [];
  error: [message: string];
  'annotation-request': [kind: AnnotationKind, selection: { quote: string; offset: number }];
  'open-reference': [href: string];
}>();

const container = ref<HTMLDivElement>();
const editorReady = ref(false);
let crepe: Crepe | undefined;
let applyingExternalChange = true;
let editorMarkdown = '';
let externalMarkdown: string | undefined = props.modelValue;
let disposed = false;
let resolveReady: () => void = () => {};
const ready = new Promise<void>((resolve) => {
  resolveReady = resolve;
});
const imageCache = new Map<string, Promise<string>>();
const mermaidRequests = new Map<
  string,
  { content: string; applyPreview: (value: null | string | HTMLElement) => void }
>();
let nextMermaidRequestId = 0;

function mermaidPreview(
  language: string,
  content: string,
  applyPreview: (value: null | string | HTMLElement) => void,
): null | string | void {
  if (language.toLowerCase() !== 'mermaid' || !content.trim()) return null;
  if (props.modelValue.length < 100_000)
    return renderMermaidPreview(language, content, applyPreview);
  const cached = getCachedMermaidPreview(content);
  if (cached) return cached;
  const id = String(++nextMermaidRequestId);
  mermaidRequests.set(id, { content, applyPreview });
  if (mermaidRequests.size > 512) {
    const oldest = mermaidRequests.keys().next().value;
    if (oldest !== undefined) mermaidRequests.delete(oldest);
  }
  const label = props.locale === 'en' ? 'Show flowchart' : '显示流程图';
  return `<button type="button" class="vivin-mermaid-load" data-vivin-mermaid-id="${id}">${label}</button>`;
}

function handleMermaidClick(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const button = target.closest<HTMLButtonElement>('[data-vivin-mermaid-id]');
  if (!button || !container.value?.contains(button)) return;
  event.preventDefault();
  event.stopPropagation();
  const id = button.dataset.vivinMermaidId;
  if (!id) return;
  const request = mermaidRequests.get(id);
  if (!request) return;
  mermaidRequests.delete(id);
  button.disabled = true;
  button.textContent = props.locale === 'en' ? 'Rendering flowchart…' : '正在生成流程图…';
  const cached = renderMermaidPreview('mermaid', request.content, request.applyPreview);
  if (typeof cached === 'string') request.applyPreview(cached);
}

function openLink(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const link = target.closest<HTMLAnchorElement>('a[href]');
  if (!link || !container.value?.contains(link)) return;
  event.preventDefault();
  event.stopPropagation();
  emit('open-reference', link.getAttribute('href') ?? '');
}

function handleRenderedClick(event: MouseEvent) {
  handleMermaidClick(event);
  if (!event.defaultPrevented) openLink(event);
}

function keepHorizontalPosition() {
  if (container.value?.scrollLeft) container.value.scrollLeft = 0;
}

async function revealAnnotation(id: string): Promise<boolean> {
  await ready;
  const node = [
    ...(container.value?.querySelectorAll<HTMLElement>('[data-annotation-id]') ?? []),
  ].find((element) => element.dataset.annotationId === id);
  if (!node) return false;
  node.scrollIntoView({ block: 'center', inline: 'nearest' });
  return true;
}

defineExpose({ revealAnnotation });

async function proxyImage(url: string): Promise<string> {
  if (/^(https?:|data:|blob:)/i.test(url)) return url;
  let cached = imageCache.get(url);
  if (!cached) {
    cached = readProjectImage(props.documentPath, decodeURI(url));
    imageCache.set(url, cached);
  }
  try {
    return await cached;
  } catch (error) {
    emit('error', String(error));
    return '';
  }
}

async function uploadImage(file: File): Promise<string> {
  try {
    return await importImage(props.documentPath, file);
  } catch (error) {
    emit('error', String(error));
    throw error;
  }
}

function occurrences(text: string, quote: string): number[] {
  const offsets: number[] = [];
  let from = 0;
  while (from < text.length) {
    const next = text.indexOf(quote, from);
    if (next < 0) break;
    offsets.push(next);
    from = next + Math.max(1, quote.length);
  }
  return offsets;
}
const annotationIcons: Record<AnnotationKind, string> = {
  todo: '<svg viewBox="0 0 24 24"><path d="m5 12 4 4 10-10" fill="none" stroke="currentColor" stroke-width="2"/></svg>',
  comment:
    '<svg viewBox="0 0 24 24"><path d="M4 5h16v11H9l-5 4V5Z" fill="none" stroke="currentColor" stroke-width="2"/></svg>',
  question:
    '<svg viewBox="0 0 24 24"><path d="M9 9a3 3 0 1 1 5 2.2c-1.3 1-2 1.5-2 3M12 18h.01" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="2"/></svg>',
  error:
    '<svg viewBox="0 0 24 24"><path d="M12 4 2 21h20L12 4Zm0 6v5m0 3h.01" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="2"/></svg>',
};
const markRefreshKey = new PluginKey<DecorationSet>('vivin-marks');
function requestAnnotation(kind: AnnotationKind, context: Ctx) {
  const view = context.get(editorViewCtx);
  const { from, to } = view.state.selection;
  const quote = view.state.doc.textBetween(from, to, '', '');
  if (!quote) return;
  const visible = view.state.doc.textBetween(0, view.state.doc.content.size, '', '');
  const prefix = view.state.doc.textBetween(0, from, '', '');
  const rank = occurrences(prefix, quote).length;
  const raw = occurrences(props.modelValue, quote);
  const matches = occurrences(visible, quote);
  const offset = raw.length === matches.length ? raw[rank] : raw.length === 1 ? raw[0] : undefined;
  if (offset === undefined) {
    emit(
      'error',
      props.locale === 'en'
        ? 'Cannot locate this selection. Try source mode.'
        : '无法准确定位选中文字，请切换原文模式重试',
    );
    return;
  }
  emit('annotation-request', kind, { quote, offset });
}

const annotationPlugin = $prose(() => {
  let cachedRendered = '';
  let textRuns: { start: number; end: number; position: number }[] = [];
  let indexedDoc: object | undefined;
  const positionAt = (offset: number): number | undefined => {
    let left = 0;
    let right = textRuns.length - 1;
    while (left <= right) {
      const middle = (left + right) >>> 1;
      const run = textRuns[middle];
      if (!run) return undefined;
      if (offset < run.start) right = middle - 1;
      else if (offset >= run.end) left = middle + 1;
      else return run.position + offset - run.start;
    }
    return undefined;
  };
  const buildDecorations = (state: EditorState): DecorationSet => {
    if (indexedDoc !== state.doc) {
      textRuns = [];
      const renderedChunks: string[] = [];
      let renderedLength = 0;
      state.doc.descendants((node, position) => {
        if (!node.isText || !node.text) return;
        textRuns.push({
          start: renderedLength,
          end: renderedLength + node.text.length,
          position,
        });
        renderedChunks.push(node.text);
        renderedLength += node.text.length;
      });
      cachedRendered = renderedChunks.join('');
      indexedDoc = state.doc;
    }
    const quotes = [
      ...new Set([...(props.annotations ?? []), ...(props.terms ?? [])].map((mark) => mark.quote)),
    ];
    const matchQuotes = createTextMatcher(quotes.map((quote) => ({ text: quote, value: quote })));
    const rawRanks = new Map<string, Map<number, number>>();
    const renderedOccurrences = new Map<string, number[]>();
    const rawEnds = new Map<string, number>();
    for (const match of matchQuotes(props.modelValue)) {
      if (match.offset < (rawEnds.get(match.value) ?? 0)) continue;
      rawEnds.set(match.value, match.offset + match.length);
      let ranks = rawRanks.get(match.value);
      if (!ranks) {
        ranks = new Map();
        rawRanks.set(match.value, ranks);
      }
      ranks.set(match.offset, ranks.size);
    }
    const renderedEnds = new Map<string, number>();
    for (const match of matchQuotes(cachedRendered)) {
      if (match.offset < (renderedEnds.get(match.value) ?? 0)) continue;
      renderedEnds.set(match.value, match.offset + match.length);
      let positions = renderedOccurrences.get(match.value);
      if (!positions) {
        positions = [];
        renderedOccurrences.set(match.value, positions);
      }
      positions.push(match.offset);
    }
    const visibleOffset = (quote: string, rawOffset: number): number | undefined => {
      const ranks = rawRanks.get(quote);
      const rendered = renderedOccurrences.get(quote);
      if (!ranks || !rendered || ranks.size !== rendered.length) return undefined;
      const rank = ranks.get(rawOffset);
      return rank === undefined ? undefined : rendered[rank];
    };
    const decorations: ReturnType<typeof Decoration.inline>[] = [];
    for (const mark of props.annotations ?? []) {
      const start = visibleOffset(mark.quote, mark.offset);
      if (start === undefined) continue;
      const from = positionAt(start);
      const last = positionAt(start + mark.quote.length - 1);
      if (from === undefined || last === undefined) continue;
      const to = last + 1;
      if (!state.doc.resolve(from).sameParent(state.doc.resolve(to))) continue;
      decorations.push(
        Decoration.inline(from, to, {
          class: `vivin-annotation vivin-annotation-${mark.kind}`,
          'data-annotation-id': mark.id,
        }),
      );
    }
    for (const mark of props.terms ?? []) {
      const start = visibleOffset(mark.quote, mark.offset);
      if (start === undefined) continue;
      const from = positionAt(start);
      const last = positionAt(start + mark.quote.length - 1);
      if (from === undefined || last === undefined) continue;
      const to = last + 1;
      if (!state.doc.resolve(from).sameParent(state.doc.resolve(to))) continue;
      decorations.push(
        Decoration.inline(from, to, {
          class: 'vivin-term',
          'data-term-id': mark.id,
          'data-term-offset': String(mark.offset),
        }),
      );
    }
    return DecorationSet.create(state.doc, decorations);
  };
  return new Plugin<DecorationSet>({
    key: markRefreshKey,
    state: {
      init: (_config, state) => buildDecorations(state),
      apply(transaction, decorations, _oldState, newState) {
        if (transaction.getMeta(markRefreshKey)) return buildDecorations(newState);
        return transaction.docChanged
          ? decorations.map(transaction.mapping, newState.doc)
          : decorations;
      },
    },
    props: {
      nodeViews: {
        image(node) {
          const image = document.createElement('img');
          const apply = (src: string, alt: string) => {
            image.alt = alt;
            void proxyImage(src).then((url) => {
              image.src = url;
            });
          };
          apply(String(node.attrs.src ?? ''), String(node.attrs.alt ?? ''));
          return {
            dom: image,
            update(next) {
              if (next.type !== node.type) return false;
              apply(String(next.attrs.src ?? ''), String(next.attrs.alt ?? ''));
              return true;
            },
            ignoreMutation: () => true,
          };
        },
      },
      decorations: (state) => markRefreshKey.getState(state) ?? DecorationSet.empty,
    },
  });
});

onMounted(async () => {
  try {
    if (!container.value) return;
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
    if (disposed) return;
    const instance = new Crepe({
      root: container.value,
      defaultValue: props.modelValue,
      features: { [CrepeFeature.ImageBlock]: false },
      featureConfigs: {
        [CrepeFeature.CodeMirror]: {
          renderPreview: mermaidPreview,
          previewOnlyByDefault: true,
          previewToggleText: (previewOnlyMode) =>
            previewOnlyMode
              ? props.locale === 'en'
                ? 'Edit code'
                : '编辑代码'
              : props.locale === 'en'
                ? 'Show preview'
                : '显示预览',
        },
        [CrepeFeature.Placeholder]: {
          text: props.locale === 'en' ? 'Start writing…' : '开始写作…',
        },
        [CrepeFeature.LinkTooltip]: {
          editButton: props.locale === 'en' ? 'Edit link' : '编辑链接',
          removeButton: props.locale === 'en' ? 'Remove link' : '移除链接',
          confirmButton: props.locale === 'en' ? 'Confirm' : '确定',
          inputPlaceholder: props.locale === 'en' ? 'Paste a link…' : '粘贴链接…',
        },
        [CrepeFeature.BlockEdit]: {
          textGroup: {
            label: props.locale === 'en' ? 'Text' : '文本',
            text: { label: props.locale === 'en' ? 'Text' : '正文' },
            h1: { label: props.locale === 'en' ? 'Heading 1' : '一级标题' },
            h2: { label: props.locale === 'en' ? 'Heading 2' : '二级标题' },
            h3: { label: props.locale === 'en' ? 'Heading 3' : '三级标题' },
            h4: { label: props.locale === 'en' ? 'Heading 4' : '四级标题' },
            h5: { label: props.locale === 'en' ? 'Heading 5' : '五级标题' },
            h6: { label: props.locale === 'en' ? 'Heading 6' : '六级标题' },
            quote: { label: props.locale === 'en' ? 'Quote' : '引用' },
            divider: { label: props.locale === 'en' ? 'Divider' : '分割线' },
          },
          listGroup: {
            label: props.locale === 'en' ? 'Lists' : '列表',
            bulletList: { label: props.locale === 'en' ? 'Bullet list' : '无序列表' },
            orderedList: { label: props.locale === 'en' ? 'Numbered list' : '有序列表' },
            taskList: { label: props.locale === 'en' ? 'Task list' : '任务列表' },
          },
          advancedGroup: {
            label: props.locale === 'en' ? 'Advanced' : '更多内容',
            image: { label: props.locale === 'en' ? 'Image' : '图片' },
            codeBlock: { label: props.locale === 'en' ? 'Code block' : '代码块' },
            table: { label: props.locale === 'en' ? 'Table' : '表格' },
            math: { label: props.locale === 'en' ? 'Math' : '公式' },
          },
        },
        [CrepeFeature.Toolbar]: {
          boldLabel: props.locale === 'en' ? 'Bold' : '加粗',
          italicLabel: props.locale === 'en' ? 'Italic' : '斜体',
          strikethroughLabel: props.locale === 'en' ? 'Strikethrough' : '删除线',
          codeLabel: props.locale === 'en' ? 'Inline code' : '行内代码',
          latexLabel: props.locale === 'en' ? 'Inline math' : '行内公式',
          linkLabel: props.locale === 'en' ? 'Link' : '链接',
          buildToolbar(builder) {
            const group = builder.addGroup(
              'annotations',
              props.locale === 'en' ? 'Annotations' : '标注',
            );
            for (const kind of ['todo', 'comment', 'question', 'error'] as const) {
              const labels = {
                todo: ['待办', 'Todo'],
                comment: ['批注', 'Comment'],
                question: ['疑问', 'Question'],
                error: ['错误', 'Error'],
              };
              group.addItem(kind, {
                icon: annotationIcons[kind],
                label: labels[kind][props.locale === 'en' ? 1 : 0],
                active: () => false,
                onRun: (context) => requestAnnotation(kind, context),
              });
            }
          },
        },
      },
    });
    instance.editor.config((context) => {
      context.update(uploadConfig.key, (previous) => ({
        ...previous,
        uploader: async (files, schema) => {
          const image = schema.nodes.image;
          if (!image) return [];
          const uploaded = await Promise.all(
            Array.from(files)
              .filter((file) => file.type.startsWith('image/'))
              .map(async (file) =>
                image.createAndFill({ src: await uploadImage(file), alt: file.name }),
              ),
          );
          return uploaded.filter((node) => node !== null);
        },
      }));
    });
    instance.editor.use(annotationPlugin);
    instance.on((listener) => {
      listener.markdownUpdated((_context, markdown) => {
        editorMarkdown = markdown;
        if (externalMarkdown !== undefined) {
          const external = externalMarkdown;
          externalMarkdown = undefined;
          if (equivalentMarkdown(markdown, external)) return;
        }
        if (!applyingExternalChange && markdown !== props.modelValue)
          emit('update:modelValue', markdown);
      });
    });
    await instance.create();
    if (disposed) {
      await instance.destroy();
      return;
    }
    const roundtrip = instance.getMarkdown();
    editorMarkdown = roundtrip;
    if (!equivalentMarkdown(roundtrip, props.modelValue)) {
      await instance.destroy();
      emit('incompatible');
      return;
    }
    instance.setReadonly(Boolean(props.readonly));
    crepe = instance;
    applyingExternalChange = false;
    editorReady.value = true;
  } finally {
    resolveReady();
  }
});

watch(
  () => props.modelValue,
  (value) => {
    if (!crepe || editorMarkdown === value) return;
    applyingExternalChange = true;
    externalMarkdown = value;
    crepe.editor.action(replaceAll(value));
    editorMarkdown = value;
    applyingExternalChange = false;
  },
);

watch(
  () => props.readonly,
  (value) => crepe?.setReadonly(Boolean(value)),
);

watch(
  () => props.annotations,
  () => {
    if (!crepe) return;
    crepe.editor.action((context) => {
      const view = context.get(editorViewCtx);
      view.dispatch(view.state.tr.setMeta(markRefreshKey, true));
    });
  },
);

watch(
  () => props.terms,
  () => {
    if (!crepe) return;
    crepe.editor.action((context) => {
      const view = context.get(editorViewCtx);
      view.dispatch(view.state.tr.setMeta(markRefreshKey, true));
    });
  },
);

onBeforeUnmount(() => {
  disposed = true;
  mermaidRequests.clear();
  resolveReady();
  void crepe?.destroy();
});
</script>

<template>
  <div class="relative h-full min-w-0">
    <div
      ref="container"
      class="rendered-editor h-full min-w-0 overflow-x-hidden overflow-y-auto px-8 py-7 transition-opacity duration-150"
      :class="[
        editorReady ? 'opacity-100' : 'opacity-0',
        modelValue.length >= 100_000 ? 'rendered-editor-large' : '',
      ]"
      @click.capture="handleRenderedClick"
      @scroll.passive="keepHorizontalPosition"
    />
    <LoadingState
      v-if="!editorReady"
      class="absolute inset-0"
      :label="locale === 'en' ? 'Preparing document…' : '正在排版文档…'"
    />
  </div>
</template>

<style>
.rendered-editor .milkdown {
  margin: 0 auto;
  max-width: 100%;
  min-height: 100%;
  box-shadow: none;
}

.rendered-editor .milkdown .ProseMirror {
  padding: 60px 8%;
  overflow-x: clip;
}

.rendered-editor-large .milkdown .ProseMirror > .milkdown-table-block {
  content-visibility: auto;
  contain-intrinsic-size: auto 360px;
}

.rendered-editor-large .milkdown .ProseMirror > .milkdown-code-block {
  content-visibility: auto;
  contain-intrinsic-size: auto 220px;
}

.rendered-editor .milkdown .milkdown-code-block .preview {
  max-width: 100%;
  overflow-x: auto;
}

.rendered-editor .milkdown .milkdown-code-block .preview svg {
  display: block;
  margin: 0 auto;
}

.rendered-editor .milkdown .vivin-mermaid-load {
  display: block;
  width: 100%;
  min-height: 72px;
  border: 1px dashed #a9b8cc;
  border-radius: 6px;
  background: #f8fafc;
  color: #2563eb;
  cursor: pointer;
}

.rendered-editor .milkdown .milkdown-code-block .preview:has(svg[id^='vivin-mermaid-']) {
  padding: 12px;
  border-radius: 6px;
  background: #fff;
}

.rendered-editor .milkdown .vivin-mermaid-error {
  padding: 12px;
  color: #b42318;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.rendered-editor .milkdown .milkdown-table-block {
  max-width: 100%;
  min-width: 0;
}

.rendered-editor .milkdown .milkdown-table-block .table-wrapper {
  display: block;
  max-width: 100%;
  overflow-x: auto;
  overflow-y: hidden;
  overscroll-behavior-x: contain;
}

.rendered-editor .milkdown .milkdown-table-block table {
  width: max-content;
  min-width: 100%;
  table-layout: auto;
}

.rendered-editor .milkdown .milkdown-table-block th,
.rendered-editor .milkdown .milkdown-table-block td {
  min-width: 170px;
  max-width: 360px;
  white-space: normal;
  overflow-wrap: break-word;
  vertical-align: top;
}
</style>
