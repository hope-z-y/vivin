<script setup lang="ts">
import type { RootContent } from 'hast';
import {
  defineComponent,
  h,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  shallowRef,
  watch,
  type PropType,
  type VNodeChild,
} from 'vue';
import type { PreviewSection } from '../lib/markdown-preview';
import { mermaidSource } from '../lib/mermaid-source';
import LoadingState from './loading-state.vue';
import MermaidDiagram from './mermaid-diagram.vue';
import PreviewImage from './preview-image.vue';
import 'katex/dist/katex.min.css';

const props = defineProps<{
  modelValue: string;
  documentPath: string;
  locale?: 'zh' | 'en';
  theme: 'light' | 'dark';
}>();
const emit = defineEmits<{ 'open-reference': [href: string]; error: [message: string] }>();
const sections = shallowRef<PreviewSection[]>([]);
const mounted = shallowRef(new Set([0, 1]));
const measured = shallowRef<Record<number, number>>({});
const host = ref<HTMLDivElement>();
const loading = ref(true);
const failure = ref('');
const generation = ref(0);
const shells = new Map<number, HTMLElement>();
const visible = new Set<number>();
let observer: IntersectionObserver | undefined;
let resizeObserver: ResizeObserver | undefined;
let worker: Worker | undefined;
let resolveReady = () => {};
let ready: Promise<void> = Promise.resolve();
let jumpTarget: number | undefined;

const PreviewNodes = defineComponent({
  props: { nodes: { type: Array as PropType<RootContent[]>, required: true } },
  setup(componentProps) {
    function renderNode(node: RootContent): VNodeChild {
      if (node.type === 'text') return node.value;
      if (node.type !== 'element') return null;
      const source = mermaidSource(node);
      if (source !== undefined)
        return h(MermaidDiagram, { content: source, theme: props.theme, locale: props.locale });
      if (node.tagName === 'img')
        return h(PreviewImage, {
          src: String(node.properties.src ?? ''),
          alt: String(node.properties.alt ?? ''),
          title: typeof node.properties.title === 'string' ? node.properties.title : undefined,
          documentPath: props.documentPath,
        });
      const attributes: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(node.properties)) {
        attributes[key === 'className' ? 'class' : key === 'htmlFor' ? 'for' : key] = Array.isArray(
          value,
        )
          ? value.join(' ')
          : value;
      }
      return h(node.tagName, attributes, node.children.map(renderNode));
    }
    return () => componentProps.nodes.map(renderNode);
  },
});

watch(
  () => props.modelValue,
  (markdown, previous, onCleanup) => {
    resolveReady();
    ready = new Promise<void>((resolve) => {
      resolveReady = resolve;
    });
    loading.value = true;
    failure.value = '';
    worker?.terminate();
    let active = true;
    const failed = (message: string) => {
      if (!active) return;
      loading.value = false;
      failure.value = message;
      emit('error', message);
      resolveReady();
    };
    let current: Worker | undefined;
    const timer = setTimeout(
      () => {
        try {
          current = new Worker(new URL('../lib/markdown-preview.worker.ts', import.meta.url), {
            type: 'module',
          });
          worker = current;
          current.onerror = (event) => failed(event.message);
          current.onmessage = (
            event: MessageEvent<{ sections?: PreviewSection[]; error?: string }>,
          ) => {
            if (!active) return;
            if (event.data.error) {
              failed(event.data.error);
              return;
            }
            sections.value = event.data.sections ?? [];
            mounted.value = new Set([0, 1]);
            measured.value = {};
            visible.clear();
            generation.value++;
            loading.value = false;
            resolveReady();
          };
          current.postMessage({ id: 0, markdown });
        } catch (error) {
          failed(String(error));
        }
      },
      previous === undefined ? 0 : 200,
    );
    onCleanup(() => {
      active = false;
      clearTimeout(timer);
      current?.terminate();
    });
  },
  { immediate: true },
);

function updateMounted() {
  const next = new Set<number>();
  if (jumpTarget !== undefined) next.add(jumpTarget);
  for (const index of visible) {
    for (let i = Math.max(0, index - 1); i <= Math.min(sections.value.length - 1, index + 1); i++)
      next.add(i);
  }
  if (next.size) mounted.value = next;
}

function setShell(value: unknown, index: number) {
  const previous = shells.get(index);
  if (previous === value) return;
  if (previous) {
    observer?.unobserve(previous);
    resizeObserver?.unobserve(previous);
  }
  if (!(value instanceof HTMLElement)) {
    shells.delete(index);
    return;
  }
  shells.set(index, value);
  observer?.observe(value);
  resizeObserver?.observe(value);
}

onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const index = Number((entry.target as HTMLElement).dataset.section);
        if (entry.isIntersecting) visible.add(index);
        else visible.delete(index);
      }
      updateMounted();
    },
    { root: host.value, rootMargin: '600px' },
  );
  resizeObserver = new ResizeObserver((entries) => {
    const next = { ...measured.value };
    let changed = false;
    for (const entry of entries) {
      const index = Number((entry.target as HTMLElement).dataset.section);
      const height = Math.ceil(entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height);
      if (mounted.value.has(index) && height > 0 && next[index] !== height) {
        next[index] = height;
        changed = true;
      }
    }
    if (changed) measured.value = next;
  });
  for (const shell of shells.values()) {
    observer.observe(shell);
    resizeObserver.observe(shell);
  }
});

onBeforeUnmount(() => {
  worker?.terminate();
  observer?.disconnect();
  resizeObserver?.disconnect();
  resolveReady();
});

async function revealInSection(index: number, selector: string): Promise<boolean> {
  if (index < 0) return false;
  jumpTarget = index;
  mounted.value = new Set([...mounted.value, index]);
  try {
    await nextTick();
    const target = shells.get(index)?.querySelector<HTMLElement>(selector);
    if (!target) return false;
    target.scrollIntoView({ block: 'center' });
    return true;
  } finally {
    jumpTarget = undefined;
  }
}

async function revealHeading(offset: number) {
  let pending: Promise<void>;
  do {
    pending = ready;
    await pending;
  } while (pending !== ready);
  const index = sections.value.findIndex(
    (section) => offset >= section.start && offset < section.end,
  );
  return revealInSection(index, `[data-source-offset="${offset}"]`);
}

function hasId(node: RootContent, id: string): boolean {
  return (
    node.type === 'element' &&
    (node.properties.id === id || node.children.some((child) => hasId(child, id)))
  );
}
async function openLink(event: MouseEvent) {
  const link = event.target instanceof Element ? event.target.closest('a[href]') : null;
  if (!link) return;
  event.preventDefault();
  const href = link.getAttribute('href') ?? '';
  if (href.startsWith('#')) {
    try {
      const id = decodeURIComponent(href.slice(1));
      const index = sections.value.findIndex((section) =>
        section.nodes.some((node) => hasId(node, id)),
      );
      await revealInSection(index, `#${CSS.escape(id)}`);
    } catch {
      /* Malformed fragments do not navigate out of the document. */
    }
  } else emit('open-reference', href);
}

defineExpose({ revealHeading, revealAnnotation: async () => false });
</script>

<template>
  <div
    ref="host"
    class="markdown-preview h-full overflow-y-auto overflow-x-hidden"
    :aria-busy="loading"
    @click="openLink"
  >
    <LoadingState
      v-if="loading && sections.length === 0"
      :label="locale === 'en' ? 'Preparing preview…' : '正在准备预览…'"
    />
    <p v-else-if="failure" class="p-8 text-sm text-red-600" role="alert">
      {{
        locale === 'en'
          ? 'Preview failed. Switch to source mode to read this document.'
          : '预览失败，请切换原文查看。'
      }}<br />{{ failure }}
    </p>
    <article
      v-else
      class="document-content preview-content"
      :aria-label="locale === 'en' ? 'Read-only Markdown preview' : 'Markdown 只读预览'"
    >
      <div
        v-for="section in sections"
        :key="`${generation}:${section.id}`"
        :ref="(value) => setShell(value, section.id)"
        :data-section="section.id"
        class="preview-section"
        :style="
          mounted.has(section.id)
            ? undefined
            : { height: `${measured[section.id] ?? section.estimatedHeight}px` }
        "
      >
        <PreviewNodes v-if="mounted.has(section.id)" :nodes="section.nodes" />
      </div>
      <p v-if="sections.length === 0" class="text-slate-400">
        {{ locale === 'en' ? 'This document is empty' : '这篇文档还没有内容' }}
      </p>
    </article>
  </div>
</template>
