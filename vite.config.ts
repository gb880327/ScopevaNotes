import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

// Tauri expects a fixed dev port; the Rust side watches src-tauri.
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'safari15',
    rollupOptions: {
      // Two windows, two entries. The panel keeps its own graph so it never
      // drags in the editor/preview bundle (design D3/D14).
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        'quick-tabs': fileURLToPath(new URL('./quick-tabs.html', import.meta.url)),
      },
    },
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
