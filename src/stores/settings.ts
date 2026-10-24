import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { load, type Store } from '@tauri-apps/plugin-store'
import { isDesktop } from '@/services/commands'

export type ThemeMode = 'light' | 'dark' | 'system'
export type Accent = 'blue' | 'purple' | 'green' | 'orange' | 'graphite'

export interface AppearanceSettings {
  theme: ThemeMode
  accent: Accent
  editorFontSize: number
  sidebarWidth: number
}

const STORAGE_KEY = 'notes.settings'
const DEFAULTS: AppearanceSettings = {
  theme: 'system',
  accent: 'blue',
  editorFontSize: 14,
  sidebarWidth: 300,
}

/**
 * UI preferences only. Note data never lives here (plan §4.3) — the Tauri
 * Store file is the desktop source of truth, localStorage is the dev fallback.
 */
export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<ThemeMode>(DEFAULTS.theme)
  const accent = ref<Accent>(DEFAULTS.accent)
  const editorFontSize = ref(DEFAULTS.editorFontSize)
  const sidebarWidth = ref(DEFAULTS.sidebarWidth)
  const ready = ref(false)

  let store: Store | null = null

  async function hydrate() {
    try {
      if (isDesktop) {
        store = await load('settings.json', { autoSave: false, defaults: {} })
        const saved = await store.get<Partial<AppearanceSettings>>('appearance')
        apply(saved)
      } else {
        const raw = localStorage.getItem(STORAGE_KEY)
        if (raw) apply(JSON.parse(raw) as Partial<AppearanceSettings>)
      }
    } catch (err) {
      console.warn('[settings] hydrate failed, using defaults', err)
    } finally {
      ready.value = true
    }
  }

  function apply(saved: Partial<AppearanceSettings> | null | undefined) {
    if (!saved) return
    if (saved.theme) theme.value = saved.theme
    if (saved.accent) accent.value = saved.accent
    if (typeof saved.editorFontSize === 'number') editorFontSize.value = saved.editorFontSize
    if (typeof saved.sidebarWidth === 'number') sidebarWidth.value = saved.sidebarWidth
  }

  function snapshot(): AppearanceSettings {
    return {
      theme: theme.value,
      accent: accent.value,
      editorFontSize: editorFontSize.value,
      sidebarWidth: sidebarWidth.value,
    }
  }

  async function persist() {
    const value = snapshot()
    try {
      if (store) {
        await store.set('appearance', value)
        await store.save()
      } else if (!isDesktop) {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
      }
    } catch (err) {
      console.warn('[settings] persist failed', err)
    }
  }

  watch([theme, accent, editorFontSize, sidebarWidth], () => void persist())

  return { theme, accent, editorFontSize, sidebarWidth, ready, hydrate }
})
