<script setup lang="ts">
import { ArrowDown, ArrowUp, ChevronDown, ChevronRight, X } from '@lucide/vue';
import { computed, nextTick, ref, watch } from 'vue';
import { findDocumentMatches, type SearchRange } from '../lib/document-search';

const props = defineProps<{
  content: string;
  readonly: boolean;
  locale: 'zh' | 'en';
}>();
const emit = defineEmits<{
  highlights: [matches: SearchRange[], active: number];
  navigate: [match: SearchRange];
  replace: [matches: SearchRange[], replacement: string];
  close: [];
}>();
const input = ref<HTMLInputElement>();
const query = ref('');
const replacement = ref('');
const replaceOpen = ref(false);
const caseSensitive = ref(false);
const wholeWord = ref(false);
const active = ref(0);
const matches = computed(() =>
  findDocumentMatches(props.content, query.value, {
    caseSensitive: caseSensitive.value,
    wholeWord: wholeWord.value,
  }),
);
const t = (zh: string, en: string) => (props.locale === 'en' ? en : zh);
watch([matches, active], () => {
  active.value = Math.min(active.value, Math.max(0, matches.value.length - 1));
  emit('highlights', matches.value, active.value);
});
watch([query, caseSensitive, wholeWord], async () => {
  active.value = 0;
  await nextTick();
  const match = matches.value[0];
  if (match) emit('navigate', match);
});
function navigate(direction: number) {
  if (!matches.value.length) return;
  active.value = (active.value + direction + matches.value.length) % matches.value.length;
  const match = matches.value[active.value];
  if (match) emit('navigate', match);
}
async function replace(all: boolean) {
  if (props.readonly) return;
  const match = matches.value[active.value];
  if (!match) return;
  emit('replace', all ? matches.value : [match], replacement.value);
  if (!all) {
    await nextTick();
    const next = matches.value.findIndex(
      (candidate) => candidate.offset >= match.offset + replacement.value.length,
    );
    active.value = Math.max(0, next);
    const current = matches.value[active.value];
    if (current) emit('navigate', current);
  }
}
function focus(selection?: string) {
  if (selection) query.value = selection;
  void nextTick(() => {
    input.value?.focus();
    input.value?.select();
  });
}
defineExpose({ focus });
</script>

<template>
  <div
    class="document-find border-b border-slate-200 bg-white p-2 text-xs"
    role="search"
    :aria-label="t('文件内搜索', 'Find in file')"
    @keydown.esc.stop.prevent="emit('close')"
  >
    <div class="flex items-center gap-1">
      <button
        :title="t('展开替换', 'Toggle replace')"
        :aria-expanded="replaceOpen"
        @click="replaceOpen = !replaceOpen"
      >
        <component :is="replaceOpen ? ChevronDown : ChevronRight" :size="15" />
      </button>
      <input
        ref="input"
        v-model="query"
        class="min-w-0 flex-1 rounded border border-slate-200 bg-transparent px-2 py-1"
        :placeholder="t('查找', 'Find')"
        :aria-label="t('文件内查找', 'Find in file')"
        @keydown.enter.stop.prevent="navigate($event.shiftKey ? -1 : 1)"
      />
      <button
        :aria-pressed="caseSensitive"
        :title="t('区分大小写', 'Match case')"
        @click="caseSensitive = !caseSensitive"
      >
        Aa
      </button>
      <button
        :aria-pressed="wholeWord"
        :title="t('全字匹配', 'Match whole word')"
        @click="wholeWord = !wholeWord"
      >
        ab
      </button>
      <span class="shrink-0 px-1" role="status">{{
        matches.length ? `${active + 1}/${matches.length}` : t('无结果', 'No results')
      }}</span>
      <button
        :disabled="!matches.length"
        :aria-label="t('上一个匹配', 'Previous match')"
        @click="navigate(-1)"
      >
        <ArrowUp :size="15" />
      </button>
      <button
        :disabled="!matches.length"
        :aria-label="t('下一个匹配', 'Next match')"
        @click="navigate(1)"
      >
        <ArrowDown :size="15" />
      </button>
      <button :aria-label="t('关闭文件搜索', 'Close find')" @click="emit('close')">
        <X :size="15" />
      </button>
    </div>
    <div v-if="replaceOpen" class="mt-1 flex items-center gap-2 pl-5">
      <input
        v-model="replacement"
        :disabled="readonly"
        class="min-w-0 flex-1 rounded border border-slate-200 bg-transparent px-2 py-1"
        :placeholder="t('替换为', 'Replace')"
        :aria-label="t('替换为', 'Replace with')"
        @keydown.enter.stop.prevent="replace(false)"
      />
      <button :disabled="readonly || !matches.length" @click="replace(false)">
        {{ t('替换', 'Replace') }}
      </button>
      <button :disabled="readonly || !matches.length" @click="replace(true)">
        {{ t('全部替换', 'Replace all') }}
      </button>
    </div>
    <p v-if="readonly && replaceOpen" class="mt-1 text-slate-500">
      {{ t('文件已锁定，无法替换', 'File is locked; replacement is disabled') }}
    </p>
  </div>
</template>

<style>
.document-find button {
  padding: 3px;
  border-radius: 4px;
}
.document-find button[aria-pressed='true'] {
  background: #dbeafe;
  color: #1d4ed8;
}
.document-find button:disabled {
  opacity: 0.4;
}
.document-find button:focus-visible {
  outline: 2px solid #3b82f6;
}
.vivin-search-match {
  background: #fde68a !important;
  color: #1e293b !important;
}
.vivin-search-active {
  background: #fb923c !important;
  color: #111827 !important;
  outline: 1px solid #c2410c;
}
</style>
