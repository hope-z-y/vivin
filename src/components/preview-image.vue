<script setup lang="ts">
import { ref, watch } from 'vue';
import { readProjectImage } from '../lib/project-api';

const props = defineProps<{ src: string; alt?: string; title?: string; documentPath: string }>();
const resolved = ref('');
watch(
  () => [props.src, props.documentPath],
  async (_value, _previous, onCleanup) => {
    let current = true;
    onCleanup(() => {
      current = false;
    });
    resolved.value = '';
    // Project images use the same native path and symlink checks as the editor.
    if (/^[a-z][a-z\d+.-]*:|^\/\//i.test(props.src)) return;
    try {
      const result = await readProjectImage(props.documentPath, decodeURI(props.src));
      if (current) resolved.value = result;
    } catch {
      /* The alt text remains visible for unavailable images. */
    }
  },
  { immediate: true },
);
</script>

<template>
  <img v-if="resolved" :src="resolved" :alt="alt" :title="title" loading="lazy" />
  <span v-else class="preview-image-placeholder" role="img" :aria-label="alt || 'Image'">{{
    alt || title || src
  }}</span>
</template>
