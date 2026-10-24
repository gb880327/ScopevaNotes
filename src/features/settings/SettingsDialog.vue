<script setup lang="ts">
import { useUiStore } from '@/stores/ui'
import AppearanceSettings from './AppearanceSettings.vue'
import ShortcutSettings from './ShortcutSettings.vue'
import StartupSettings from './StartupSettings.vue'
import LibrarySettings from './LibrarySettings.vue'

const ui = useUiStore()
</script>

<template>
  <div v-if="ui.settingsOpen" class="overlay" @click.self="ui.settingsOpen = false">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="设置">
      <header>
        <h2>设置</h2>
        <button type="button" class="close" @click="ui.settingsOpen = false">×</button>
      </header>
      <div class="body">
        <AppearanceSettings />
        <hr />
        <ShortcutSettings />
        <hr />
        <StartupSettings />
        <hr />
        <section class="section">
          <h3>迁移</h3>
          <p>从其他应用（例如 Obsidian）的目录一次性导入 Markdown 文件。</p>
          <button type="button" @click="ui.openImport()">导入 Markdown…</button>
        </section>
        <hr />
        <LibrarySettings />
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}

.dialog {
  width: 520px;
  max-height: 80vh;
  overflow-y: auto;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}

h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.close {
  border: 0;
  background: none;
  font-size: 16px;
  color: var(--text-muted);
  cursor: pointer;
}

.body {
  padding: 16px 18px 20px;
}

hr {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 16px 0;
}

.section h3 {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 600;
}

.section p {
  margin: 0 0 10px;
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-muted);
}

.section button {
  font: inherit;
  font-size: 12px;
  padding: 6px 12px;
  border-radius: 7px;
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text);
  cursor: pointer;
}
</style>
