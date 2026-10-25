<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X } from '@lucide/vue';

const isMac = navigator.userAgent.includes('Macintosh');
defineProps<{ sidebarOpen: boolean; locale?: 'zh' | 'en' }>();

async function runWindowAction(action: 'close' | 'minimize' | 'toggleMaximize') {
  if (!isTauri()) return;
  await getCurrentWindow()[action]();
}
</script>

<template>
  <header class="flex h-13 shrink-0 items-center bg-[#f3f6fa] text-slate-800">
    <div
      :class="[
        'flex h-full shrink-0 items-center px-4',
        sidebarOpen ? 'w-64 justify-between' : 'w-28 justify-start gap-5',
      ]"
      data-tauri-drag-region
    >
      <div v-if="isMac" class="flex items-center gap-2">
        <button
          :aria-label="locale === 'en' ? 'Close window' : '关闭窗口'"
          :title="locale === 'en' ? 'Close window' : '关闭窗口'"
          class="h-3 w-3 rounded-full bg-[#ff5f57]"
          type="button"
          @click="runWindowAction('close')"
        />
        <button
          :aria-label="locale === 'en' ? 'Minimize window' : '最小化窗口'"
          :title="locale === 'en' ? 'Minimize window' : '最小化窗口'"
          class="h-3 w-3 rounded-full bg-[#febc2e]"
          type="button"
          @click="runWindowAction('minimize')"
        />
        <button
          :aria-label="locale === 'en' ? 'Maximize window' : '最大化窗口'"
          :title="locale === 'en' ? 'Maximize window' : '最大化窗口'"
          class="h-3 w-3 rounded-full bg-[#28c840]"
          type="button"
          @click="runWindowAction('toggleMaximize')"
        />
      </div>
      <slot name="sidebar-control" />
    </div>

    <div class="flex h-full min-w-0 flex-1 items-center justify-between gap-3 px-4">
      <slot />
    </div>

    <div v-if="!isMac" class="flex h-full items-center">
      <button
        :aria-label="locale === 'en' ? 'Minimize window' : '最小化窗口'"
        :title="locale === 'en' ? 'Minimize window' : '最小化窗口'"
        class="flex h-full w-11 items-center justify-center hover:bg-slate-200/70"
        type="button"
        @click="runWindowAction('minimize')"
      >
        <Minus :size="16" />
      </button>
      <button
        :aria-label="locale === 'en' ? 'Maximize window' : '最大化窗口'"
        :title="locale === 'en' ? 'Maximize window' : '最大化窗口'"
        class="flex h-full w-11 items-center justify-center hover:bg-slate-200/70"
        type="button"
        @click="runWindowAction('toggleMaximize')"
      >
        <Square :size="14" />
      </button>
      <button
        :aria-label="locale === 'en' ? 'Close window' : '关闭窗口'"
        :title="locale === 'en' ? 'Close window' : '关闭窗口'"
        class="flex h-full w-11 items-center justify-center hover:bg-red-500 hover:text-white"
        type="button"
        @click="runWindowAction('close')"
      >
        <X :size="17" />
      </button>
    </div>
  </header>
</template>
