<script setup lang="ts">
import { onBeforeUnmount, ref, shallowRef, watch } from 'vue';
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
  if (updateTimer) clearTimeout(updateTimer);
});
</script>

<template>
  <nav
    :id="panelId"
    :aria-label="locale === 'en' ? 'Document outline' : '文档目录'"
    class="flex h-full min-h-0 w-52 max-w-[35%] shrink-0 flex-col overflow-hidden border-r border-slate-200 bg-white"
    @click.stop
  >
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
