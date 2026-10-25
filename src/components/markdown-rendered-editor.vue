<script setup lang="ts">
import { Crepe, CrepeFeature } from '@milkdown/crepe';
import { editorViewCtx } from '@milkdown/kit/core';
import type { Ctx } from '@milkdown/kit/ctx';
import { uploadConfig } from '@milkdown/kit/plugin/upload';
import { Plugin } from '@milkdown/kit/prose/state';
import { Decoration, DecorationSet } from '@milkdown/kit/prose/view';
import { $prose, replaceAll } from '@milkdown/kit/utils';
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { AnnotationMark } from '../lib/annotation-anchor';
import type { AnnotationKind } from '../lib/project-api';
import { equivalentMarkdown } from '../lib/markdown-roundtrip';
import { importImage, readProjectImage } from '../lib/project-api';
import '@milkdown/crepe/theme/common/style.css';
import '@milkdown/crepe/theme/frame.css';

const props = defineProps<{
  modelValue: string;
  documentPath: string;
  readonly?: boolean;
  annotations?: AnnotationMark[];
  locale?: 'zh' | 'en';
}>();
const emit = defineEmits<{
  'update:modelValue': [value: string];
  incompatible: [];
  error: [message: string];
  'annotation-request': [kind: AnnotationKind, selection: { quote: string; offset: number }];
}>();

const container = ref<HTMLDivElement>();
let crepe: Crepe | undefined;
let applyingExternalChange = true;
let disposed = false;
let resolveReady: () => void = () => {};
const ready = new Promise<void>((resolve) => {
  resolveReady = resolve;
});
const imageCache = new Map<string, Promise<string>>();

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

const annotationPlugin = $prose(
  () =>
    new Plugin({
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
        decorations(state) {
          const positions: number[] = [];
          let rendered = '';
          state.doc.descendants((node, position) => {
            if (!node.isText || !node.text) return;
            rendered += node.text;
            for (let index = 0; index < node.text.length; index++) positions.push(position + index);
          });
          const decorations: ReturnType<typeof Decoration.inline>[] = [];
          for (const mark of props.annotations ?? []) {
            const rawMatches = occurrences(props.modelValue, mark.quote);
            const renderedMatches = occurrences(rendered, mark.quote);
            if (rawMatches.length !== renderedMatches.length) continue;
            const rank = rawMatches.indexOf(mark.offset);
            const start = rank < 0 ? undefined : renderedMatches[rank];
            if (start === undefined) continue;
            const from = positions[start];
            const last = positions[start + mark.quote.length - 1];
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
          return DecorationSet.create(state.doc, decorations);
        },
      },
    }),
);

onMounted(async () => {
  try {
    if (!container.value) return;
    const instance = new Crepe({
      root: container.value,
      defaultValue: props.modelValue,
      features: { [CrepeFeature.ImageBlock]: false },
      featureConfigs: {
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
        if (!applyingExternalChange && !equivalentMarkdown(markdown, props.modelValue))
          emit('update:modelValue', markdown);
      });
    });
    await instance.create();
    if (disposed) {
      await instance.destroy();
      return;
    }
    const roundtrip = instance.getMarkdown();
    if (!equivalentMarkdown(roundtrip, props.modelValue)) {
      await instance.destroy();
      emit('incompatible');
      return;
    }
    instance.setReadonly(Boolean(props.readonly));
    crepe = instance;
    applyingExternalChange = false;
  } finally {
    resolveReady();
  }
});

watch(
  () => props.modelValue,
  (value) => {
    if (!crepe || crepe.getMarkdown() === value) return;
    applyingExternalChange = true;
    crepe.editor.action(replaceAll(value));
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
      view.dispatch(view.state.tr);
    });
  },
  { deep: true },
);

onBeforeUnmount(() => {
  disposed = true;
  resolveReady();
  void crepe?.destroy();
});
</script>

<template>
  <div ref="container" class="rendered-editor h-full overflow-auto px-8 py-7" />
</template>

<style>
.rendered-editor .milkdown {
  margin: 0 auto;
  max-width: 860px;
  min-height: 100%;
  box-shadow: none;
}

.rendered-editor .milkdown .ProseMirror {
  padding: 60px 8%;
}
</style>
