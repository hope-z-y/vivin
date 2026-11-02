<script setup lang="ts">
import { Maximize2 } from '@lucide/vue';
import { ref, watch } from 'vue';
import { renderMermaidSvg } from '../lib/mermaid-preview';
import DiagramViewer from './diagram-viewer.vue';

const props = defineProps<{ content: string; theme: 'light' | 'dark'; locale?: 'zh' | 'en' }>();
const svg = ref('');
const error = ref('');
const expanded = ref(false);
watch(
  () => [props.content, props.theme],
  async (_value, _previous, onCleanup) => {
    let current = true;
    onCleanup(() => {
      current = false;
    });
    svg.value = '';
    error.value = '';
    try {
      const result = await renderMermaidSvg(props.content, props.theme === 'dark');
      if (current) svg.value = result;
    } catch (reason) {
      if (current) error.value = String(reason);
    }
  },
  { immediate: true },
);
</script>

<template>
  <figure class="mermaid-figure">
    <figcaption class="diagram-toolbar">
      <span class="flex-1 text-xs font-medium tracking-wide">MERMAID</span>
      <button
        v-if="svg"
        :title="locale === 'en' ? 'Expand diagram' : '放大查看'"
        @click="expanded = true"
      >
        <Maximize2 :size="15" /><span>{{ locale === 'en' ? 'Expand' : '放大查看' }}</span>
      </button>
    </figcaption>
    <p v-if="error" class="diagram-error">{{ error }}</p>
    <p v-else-if="!svg" class="diagram-loading" role="status">
      {{ locale === 'en' ? 'Rendering diagram…' : '正在生成图表…' }}
    </p>
    <!-- Only strict-mode Mermaid output is inserted, never raw document HTML. -->
    <div v-else class="diagram-svg" v-html="svg" />
    <details v-if="error" class="px-4 pb-4 text-xs">
      <summary>{{ locale === 'en' ? 'Diagram source' : '查看图表源码' }}</summary>
      <pre>{{ content }}</pre>
    </details>
    <DiagramViewer v-if="expanded && svg" :svg="svg" :locale="locale" @close="expanded = false" />
  </figure>
</template>
