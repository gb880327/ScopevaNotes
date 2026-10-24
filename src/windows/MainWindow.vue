<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { message } from '@tauri-apps/plugin-dialog'
import Sidebar from '@/components/layout/Sidebar.vue'
import SidebarSplitter from '@/components/layout/SidebarSplitter.vue'
import ContentPane from '@/components/layout/ContentPane.vue'
import SettingsDialog from '@/features/settings/SettingsDialog.vue'
import ImportDialog from '@/features/import/ImportDialog.vue'
import { useNotesStore } from '@/stores/notes'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'
import { useTheme } from '@/composables/useTheme'
import { useKeyboard } from '@/composables/useKeyboard'
import { api, errorMessage, isDesktop } from '@/services/commands'

const notes = useNotesStore()
const settings = useSettingsStore()
const ui = useUiStore()

const disposeTheme = useTheme()
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
      listen('ui://import-markdown', () => ui.openImport()),
      listen('ui://quit-requested', () => void handleQuitRequest()),
    ])
  }
})

/// Flush first, confirm second: the dialog must never show before the pending
/// edit is on disk, and a failed save keeps the app alive.
async function handleQuitRequest() {
  try {
    await notes.flush()
  } catch (err) {
    await message(`保存失败，已取消退出：${errorMessage(err)}`, {
      title: '退出 Notes',
      kind: 'error',
    })
    return
  }
  if (notes.saveStatus === 'error') {
    await message(`保存失败，已取消退出：${notes.error ?? '未知错误'}`, {
      title: '退出 Notes',
      kind: 'error',
    })
    return
  }
  await api.requestQuit()
}

onBeforeUnmount(() => {
  disposeTheme()
  for (const stop of unlisten) stop()
})
</script>

<template>
  <div class="shell">
    <Sidebar :style="{ width: `${settings.sidebarWidth}px` }" />
    <SidebarSplitter />
    <ContentPane />
    <SettingsDialog />
    <ImportDialog />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  height: 100%;
  min-height: 0;
}
</style>
