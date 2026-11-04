<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import { markdownHeadings, type MarkdownHeading } from '../lib/markdown-outline';

const props = defineProps<{
  markdown: string;
  locale: 'zh' | 'en';
  panelId: string;
}>();
const emit = defineEmits<{ select: [offset: number] }>();
const headings = shallowRef<MarkdownHeading[]>([]);
const loading = ref(true);
let updateTimer: ReturnType<typeof setTimeout> | undefined;
const panel = ref<HTMLElement | null>(null);
const preferredWidth = ref(readOutlineWidth());
const maxWidth = ref(480);
const minWidth = computed(() => Math.min(160, maxWidth.value));
const width = computed(() =>
  Math.max(minWidth.value, Math.min(maxWidth.value, preferredWidth.value)),
);
let resizeObserver: ResizeObserver | undefined;
let stopResizing: (() => void) | undefined;

function readOutlineWidth(): number {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem('vivin.outline-width') ?? 'null');
    return typeof saved === 'number' && Number.isFinite(saved)
      ? Math.max(160, Math.min(480, saved))
      : 208;
  } catch {
    return 208;
  }
}
function setWidth(next: number) {
  preferredWidth.value = Math.round(Math.max(minWidth.value, Math.min(maxWidth.value, next)));
  localStorage.setItem('vivin.outline-width', JSON.stringify(preferredWidth.value));
}
function resize(event: PointerEvent) {
  if (event.button !== 0) return;
  const handle = event.currentTarget;
  if (!(handle instanceof HTMLElement)) return;
  const startX = event.clientX;
  const startWidth = width.value;
  stopResizing?.();
  handle.setPointerCapture(event.pointerId);
  const move = (next: PointerEvent) => setWidth(startWidth + next.clientX - startX);
  const stop = () => {
    handle.removeEventListener('pointermove', move);
    handle.removeEventListener('pointerup', stop);
    handle.removeEventListener('pointercancel', stop);
    handle.removeEventListener('lostpointercapture', stop);
    if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
    stopResizing = undefined;
  };
  stopResizing = stop;
  handle.addEventListener('pointermove', move);
  handle.addEventListener('pointerup', stop);
  handle.addEventListener('pointercancel', stop);
  handle.addEventListener('lostpointercapture', stop);
}
onMounted(() => {
  const host = panel.value?.parentElement;
  if (!host) return;
  const updateBounds = () => {
    maxWidth.value = Math.max(120, Math.min(480, Math.floor(host.clientWidth * 0.6)));
  };
  updateBounds();
  resizeObserver = new ResizeObserver(updateBounds);
  resizeObserver.observe(host);
});

watch(
  () => props.markdown,
  (markdown, previous) => {
    if (updateTimer) clearTimeout(updateTimer);
    if (previous === undefined) loading.value = true;
    updateTimer = setTimeout(
      () => {
        headings.value = markdownHeadings(markdown);
        loading.value = false;
        updateTimer = undefined;
      },
      previous === undefined ? 0 : 250,
    );
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  stopResizing?.();
  resizeObserver?.disconnect();
  if (updateTimer) clearTimeout(updateTimer);
});
</script>

<template>
  <nav
    :id="panelId"
    ref="panel"
    :aria-label="locale === 'en' ? 'Document outline' : '文档目录'"
    class="relative flex h-full min-h-0 max-w-[60%] shrink-0 flex-col overflow-hidden border-r border-slate-200 bg-white"
    :style="{ width: `${width}px` }"
    @click.stop
  >
    <div
      class="group absolute inset-y-0 right-0 z-10 flex w-2 touch-none cursor-col-resize items-center justify-center focus-visible:outline-2 focus-visible:outline-blue-500"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      :aria-label="locale === 'en' ? 'Resize document outline' : '调整文档目录宽度'"
      :aria-controls="panelId"
      :aria-valuemin="minWidth"
      :aria-valuemax="maxWidth"
      :aria-valuenow="width"
      @pointerdown.prevent.stop="resize"
      @keydown.left.prevent="setWidth(width - 16)"
      @keydown.right.prevent="setWidth(width + 16)"
      @keydown.home.prevent="setWidth(minWidth)"
      @keydown.end.prevent="setWidth(maxWidth)"
    >
      <span
        class="h-12 w-0.5 rounded-full bg-transparent group-hover:bg-blue-400 group-focus-visible:bg-blue-400"
      />
    </div>
    <div class="shrink-0 px-4 py-3 text-xs font-semibold text-slate-500">
      {{ locale === 'en' ? 'Outline' : '目录' }}
      <span v-if="!loading" class="ml-1 text-slate-400">{{ headings.length }}</span>
    </div>
    <div class="min-h-0 overflow-y-auto px-1 pb-2">
      <p v-if="loading" class="px-3 py-4 text-xs text-slate-400">
        {{ locale === 'en' ? 'Loading headings…' : '正在生成目录…' }}
      </p>
      <p v-else-if="headings.length === 0" class="px-3 py-4 text-xs text-slate-400">
        {{ locale === 'en' ? 'No headings in this document' : '这篇文档没有标题' }}
      </p>
      <button
        v-for="heading in headings"
        :key="heading.offset"
        class="outline-heading flex w-full items-baseline gap-2 rounded-lg py-2 pr-3 text-left text-sm text-slate-700 hover:bg-slate-100 focus-visible:bg-slate-100"
        :class="heading.depth <= 2 ? 'font-semibold' : ''"
        :style="{ paddingLeft: `${12 + (heading.depth - 1) * 14}px` }"
        :title="heading.text"
        @click="emit('select', heading.offset)"
      >
        <span class="shrink-0 text-[10px] font-medium text-slate-400">H{{ heading.depth }}</span>
        <span class="min-w-0 truncate">{{
          heading.text || (locale === 'en' ? 'Untitled heading' : '无标题')
        }}</span>
      </button>
    </div>
  </nav>
</template>
