<script setup lang="ts">
defineProps<{ label: string }>();
</script>

<template>
  <div class="flex flex-col items-center px-3 py-9 text-center text-sm text-slate-400">
    <img src="/empty-list.svg" alt="" class="mb-3 h-24 w-32 object-contain" />
    <p class="leading-6">{{ label }}</p>
  </div>
</template>
