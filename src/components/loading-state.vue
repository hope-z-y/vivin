<script setup lang="ts">
import { LoaderCircle } from '@lucide/vue';

withDefaults(defineProps<{ label?: string }>(), { label: '正在加载编辑器…' });
</script>

<template>
  <div
    class="flex h-full min-h-36 flex-1 items-center justify-center gap-2 text-sm text-slate-400"
    role="status"
  >
    <LoaderCircle :size="17" class="animate-spin text-blue-600" />
    <span>{{ label }}</span>
  </div>
</template>
