<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import Sidebar from '@/components/layout/Sidebar.vue'
import SidebarSplitter from '@/components/layout/SidebarSplitter.vue'
import ContentPane from '@/components/layout/ContentPane.vue'
import SettingsDialog from '@/features/settings/SettingsDialog.vue'
import { useNotesStore } from '@/stores/notes'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'
import { useTheme } from '@/composables/useTheme'
import { useKeyboard } from '@/composables/useKeyboard'
import { isDesktop } from '@/services/commands'

const notes = useNotesStore()
const settings = useSettingsStore()
const ui = useUiStore()

useTheme()
useKeyboard({ focusSearch: () => ui.focusSearch() })

let unlisten: UnlistenFn[] = []

onMounted(async () => {
  await settings.hydrate()
  await settings.applyShortcut()
  await notes.refresh()
  await notes.refreshTags()

  if (isDesktop) {
    unlisten = await Promise.all([
      listen('ui://new-note', () => void notes.createNote()),
      listen('ui://open-settings', () => ui.openSettings()),
      listen('ui://open-library', () => ui.openSettings()),
    ])
  }
})

onBeforeUnmount(() => {
  for (const stop of unlisten) stop()
})
</script>

<template>
  <div class="shell">
    <Sidebar :style="{ width: `${settings.sidebarWidth}px` }" />
    <SidebarSplitter />
    <ContentPane />
    <SettingsDialog />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  height: 100%;
  min-height: 0;
}
</style>
