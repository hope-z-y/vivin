import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [vue(), tailwindcss()],
  clearScreen: false,
  resolve: {
    alias: {
      // The browser export uses document.createElement, unavailable in workers.
      // The default export decodes the same entities using a static map.
      'decode-named-character-reference': fileURLToPath(
        import.meta.resolve('decode-named-character-reference'),
      ),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
}));
