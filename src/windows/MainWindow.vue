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
    <!-- 无标题栏窗口的拖拽区：左段与侧栏同宽同色，右段与正文同色（design D2） -->
    <div class="titlebar">
      <span
        class="titlebar-side"
        :style="{ width: `${settings.sidebarWidth}px` }"
        data-tauri-drag-region
      />
      <span class="titlebar-main" data-tauri-drag-region />
    </div>

    <div class="body">
      <Sidebar :style="{ width: `${settings.sidebarWidth}px` }" />
      <SidebarSplitter />
      <ContentPane />
    </div>

    <SettingsDialog />
    <ImportDialog />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

/* The window has no title bar (`titleBarStyle: Overlay` + `hiddenTitle`), so
   this strip is what the user drags. It also keeps the traffic lights clear of
   the sidebar header. */
.titlebar {
  display: flex;
  flex: 0 0 auto;
  height: 32px;
}

.titlebar-side {
  flex: 0 0 auto;
  box-sizing: border-box;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border);
}

.titlebar-main {
  flex: 1;
  min-width: 0;
  background: var(--bg);
}

.body {
  display: flex;
  flex: 1;
  min-height: 0;
}
</style>
