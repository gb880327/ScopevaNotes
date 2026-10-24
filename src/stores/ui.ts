import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ViewMode = 'read' | 'edit'

/** Ephemeral window/UI state; nothing here is persisted. */
export const useUiStore = defineStore('ui', () => {
  const viewMode = ref<ViewMode>('read')
  const settingsOpen = ref(false)
  const importOpen = ref(false)
  const searchFocusToken = ref(0)

  function focusSearch() {
    searchFocusToken.value += 1
  }

  function openSettings() {
    settingsOpen.value = true
  }

  function openImport() {
    settingsOpen.value = false
    importOpen.value = true
  }

  function toggleEdit() {
    viewMode.value = viewMode.value === 'edit' ? 'read' : 'edit'
  }

  return {
    viewMode,
    settingsOpen,
    importOpen,
    searchFocusToken,
    focusSearch,
    openSettings,
    openImport,
    toggleEdit,
  }
})
