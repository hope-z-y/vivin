<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import type { AnnotationMark } from '../lib/annotation-anchor';
import {
  joinMarkdown,
  markdownSegmentRanges,
  splitMarkdown,
  type MarkdownSegment,
} from '../lib/markdown-segments';
import type { AnnotationKind } from '../lib/project-api';
import MarkdownRenderedEditor from './markdown-rendered-editor.vue';

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

type EditorHandle = {
  revealAnnotation: (id: string) => Promise<boolean>;
  revealHeading: (offset: number) => Promise<boolean>;
};
const segments = shallowRef<MarkdownSegment[]>(splitMarkdown(props.modelValue));
const scrollHost = ref<HTMLDivElement>();
const mounted = shallowRef(new Set([0, 1]));
const measured = shallowRef<Record<number, number>>({});
const shells: (HTMLDivElement | null)[] = [];
const editors: (EditorHandle | null)[] = [];
const shellRefs: ((value: unknown) => void)[] = [];
const editorRefs: ((value: unknown) => void)[] = [];
const ready = shallowRef(new Set<number>());
const visible = new Set<number>();
const recentlyEdited: number[] = [];
let headingJumpTarget: number | null = null;
let intersectionObserver: IntersectionObserver | undefined;
let resizeObserver: ResizeObserver | undefined;
let lastEmitted: string | undefined;

const markRanges = shallowRef(markdownSegmentRanges(segments.value));
const currentRanges = computed(() => markdownSegmentRanges(segments.value));

function marksBySegment<T extends { offset: number; quote: string }>(
  marks: readonly T[] | undefined,
): T[][] {
  const result = markRanges.value.map(() => [] as T[]);
  for (const mark of marks ?? []) {
    const index = markRanges.value.findIndex(
      (range) => mark.offset >= range.start && mark.offset + mark.quote.length <= range.end,
    );
    const range = markRanges.value[index];
    if (index < 0 || !range) continue;
    result[index]?.push({ ...mark, offset: mark.offset - range.start });
  }
  return result;
}

const annotationSegments = computed(() => marksBySegment(props.annotations));
const termSegments = computed(() => marksBySegment(props.terms));

watch(
  () => [props.annotations, props.terms],
  () => {
    markRanges.value = markdownSegmentRanges(segments.value);
  },
  { flush: 'sync' },
);

function setShell(value: unknown, index: number) {
  const element = value instanceof HTMLDivElement ? value : null;
  if (shells[index] === element) return;
  if (shells[index]) {
    intersectionObserver?.unobserve(shells[index]);
    resizeObserver?.unobserve(shells[index]);
  }
  shells[index] = element;
  if (element) {
    intersectionObserver?.observe(element);
    resizeObserver?.observe(element);
  }
}

function shellRef(index: number) {
  return (shellRefs[index] ??= (value) => setShell(value, index));
}

function setEditor(value: unknown, index: number) {
  editors[index] = value as EditorHandle | null;
  if (!value && ready.value.has(index)) {
    const next = new Set(ready.value);
    next.delete(index);
    ready.value = next;
  }
}

function editorRef(index: number) {
  return (editorRefs[index] ??= (value) => setEditor(value, index));
}

function updateMounted() {
  if (!visible.size) return;
  const next = new Set<number>(recentlyEdited);
  if (headingJumpTarget !== null) next.add(headingJumpTarget);
  for (const index of visible)
    for (
      let nearby = Math.max(0, index - 1);
      nearby <= Math.min(segments.value.length - 1, index + 1);
      nearby++
    )
      next.add(nearby);
  if (next.size === mounted.value.size && [...next].every((index) => mounted.value.has(index)))
    return;
  mounted.value = next;
}

onMounted(() => {
  intersectionObserver = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const index = Number((entry.target as HTMLElement).dataset.segmentIndex);
        if (!Number.isInteger(index)) continue;
        if (entry.isIntersecting) visible.add(index);
        else visible.delete(index);
      }
      updateMounted();
    },
    { root: scrollHost.value, rootMargin: '800px' },
  );
  resizeObserver = new ResizeObserver((entries) => {
    let next: Record<number, number> | undefined;
    for (const entry of entries) {
      const index = Number((entry.target as HTMLElement).dataset.segmentIndex);
      if (!ready.value.has(index)) continue;
      const height = Math.ceil(entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height);
      if (height < 1 || measured.value[index] === height) continue;
      next ??= { ...measured.value };
      next[index] = height;
    }
    if (next) measured.value = next;
  });
  for (const shell of shells) {
    if (!shell) continue;
    intersectionObserver.observe(shell);
    resizeObserver.observe(shell);
  }
});

onBeforeUnmount(() => {
  intersectionObserver?.disconnect();
  resizeObserver?.disconnect();
});

watch(
  () => props.modelValue,
  (value) => {
    if (value === lastEmitted) {
      lastEmitted = undefined;
      return;
    }
    segments.value = splitMarkdown(value);
    markRanges.value = markdownSegmentRanges(segments.value);
    mounted.value = new Set([0, 1]);
    measured.value = {};
    visible.clear();
    recentlyEdited.length = 0;
    ready.value = new Set();
  },
);

function updateSegment(index: number, content: string) {
  const previous = segments.value[index];
  if (!previous || previous.content === content) return;
  const next = [...segments.value];
  next[index] = { ...previous, content, serializedContent: content.trimEnd() };
  segments.value = next;
  const earlier = recentlyEdited.indexOf(index);
  if (earlier >= 0) recentlyEdited.splice(earlier, 1);
  recentlyEdited.push(index);
  if (recentlyEdited.length > 3) recentlyEdited.shift();
  updateMounted();
  lastEmitted = joinMarkdown(next);
  emit('update:modelValue', lastEmitted);
}

function requestAnnotation(
  index: number,
  kind: AnnotationKind,
  selection: { quote: string; offset: number },
) {
  const start = currentRanges.value[index]?.start;
  if (start === undefined) return;
  emit('annotation-request', kind, { ...selection, offset: start + selection.offset });
}

function onEditorReady(index: number) {
  ready.value = new Set([...ready.value, index]);
  const shell = shells[index];
  if (shell) resizeObserver?.observe(shell);
}

async function revealAnnotation(id: string): Promise<boolean> {
  const index = annotationSegments.value.findIndex((marks) => marks.some((mark) => mark.id === id));
  if (index < 0) return false;
  mounted.value = new Set([...mounted.value, index]);
  await nextTick();
  shells[index]?.scrollIntoView({ block: 'center', inline: 'nearest' });
  await nextTick();
  return (await editors[index]?.revealAnnotation(id)) ?? false;
}

async function revealHeading(offset: number): Promise<boolean> {
  const index = currentRanges.value.findIndex(
    (range) => offset >= range.start && offset < range.end,
  );
  if (index < 0) return false;
  headingJumpTarget = index;
  mounted.value = new Set([...mounted.value, index]);
  try {
    await nextTick();
    shells[index]?.scrollIntoView({ block: 'center', inline: 'nearest' });
    await nextTick();
    const start = currentRanges.value[index]?.start;
    if (start === undefined) return false;
    return (await editors[index]?.revealHeading(offset - start)) ?? false;
  } finally {
    headingJumpTarget = null;
    updateMounted();
  }
}

defineExpose({ revealAnnotation, revealHeading });
</script>

<template>
  <div ref="scrollHost" class="h-full min-w-0 overflow-x-hidden overflow-y-auto px-8 py-7">
    <div
      v-for="(segment, index) in segments"
      :key="segment.id"
      :ref="shellRef(index)"
      :data-segment-index="index"
      :data-segment-start="currentRanges[index]?.start"
      :data-segment-end="currentRanges[index]?.end"
      class="min-w-0"
      :style="
        mounted.has(index) && ready.has(index)
          ? undefined
          : { minHeight: `${measured[index] ?? segment.estimatedHeight}px` }
      "
    >
      <MarkdownRenderedEditor
        v-if="mounted.has(index)"
        :ref="editorRef(index)"
        :model-value="segment.content"
        :document-path="documentPath"
        :readonly="readonly"
        :annotations="annotationSegments[index]"
        :terms="termSegments[index]"
        :locale="locale"
        embedded
        defer-mermaid
        @ready="onEditorReady(index)"
        @update:model-value="updateSegment(index, $event)"
        @incompatible="emit('incompatible')"
        @error="emit('error', $event)"
        @open-reference="emit('open-reference', $event)"
        @annotation-request="(kind, selection) => requestAnnotation(index, kind, selection)"
      />
      <div v-else :style="{ height: `${measured[index] ?? segment.estimatedHeight}px` }" />
    </div>
  </div>
</template>
