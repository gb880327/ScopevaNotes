<script setup lang="ts">
import { useUiStore } from '@/stores/ui'
import AppearanceSettings from './AppearanceSettings.vue'
import ShortcutSettings from './ShortcutSettings.vue'
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
</style>
