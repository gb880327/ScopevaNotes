import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { load, type Store } from '@tauri-apps/plugin-store'
import { api, errorMessage, isDesktop } from '@/services/commands'

export type ThemeMode = 'light' | 'dark' | 'system'
export type Accent = 'blue' | 'purple' | 'green' | 'orange' | 'graphite'

export const DEFAULT_SHORTCUT = 'CommandOrControl+Shift+D'

export interface PersistedSettings {
  theme: ThemeMode
  accent: Accent
  editorFontSize: number
  sidebarWidth: number
  shortcut: string
}

const STORAGE_KEY = 'notes.settings'
/** Key kept as `appearance` for existing installs; now holds all prefs. */
const STORE_KEY = 'appearance'
const DEFAULTS: PersistedSettings = {
  theme: 'system',
  accent: 'blue',
  editorFontSize: 14,
  sidebarWidth: 300,
  shortcut: DEFAULT_SHORTCUT,
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
  const shortcut = ref(DEFAULTS.shortcut)
  const shortcutError = ref<string | null>(null)
  const ready = ref(false)
  /** Tracks `prefers-color-scheme`, so `system` can resolve without a DOM read. */
  const systemDark = ref(false)

  const resolvedTheme = computed<'light' | 'dark'>(() =>
    theme.value === 'system' ? (systemDark.value ? 'dark' : 'light') : theme.value,
  )

  let store: Store | null = null

  async function hydrate() {
    try {
      if (isDesktop) {
        store = await load('settings.json', { autoSave: false, defaults: {} })
        const saved = await store.get<Partial<PersistedSettings>>(STORE_KEY)
        apply(saved)
      } else {
        const raw = localStorage.getItem(STORAGE_KEY)
        if (raw) apply(JSON.parse(raw) as Partial<PersistedSettings>)
      }
    } catch (err) {
      console.warn('[settings] hydrate failed, using defaults', err)
    } finally {
      ready.value = true
    }
  }

  function apply(saved: Partial<PersistedSettings> | null | undefined) {
    if (!saved) return
    if (saved.theme) theme.value = saved.theme
    if (saved.accent) accent.value = saved.accent
    if (typeof saved.editorFontSize === 'number') editorFontSize.value = saved.editorFontSize
    if (typeof saved.sidebarWidth === 'number') sidebarWidth.value = saved.sidebarWidth
    if (saved.shortcut) shortcut.value = saved.shortcut
  }

  function snapshot(): PersistedSettings {
    return {
      theme: theme.value,
      accent: accent.value,
      editorFontSize: editorFontSize.value,
      sidebarWidth: sidebarWidth.value,
      shortcut: shortcut.value,
    }
  }

  async function persist() {
    const value = snapshot()
    try {
      if (store) {
        await store.set(STORE_KEY, value)
        await store.save()
      } else if (!isDesktop) {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
      }
    } catch (err) {
      console.warn('[settings] persist failed', err)
    }
  }

  /**
   * Re-applies the stored accelerator at startup. If it is no longer
   * registrable (another app took it), fall back to the default rather than
   * leaving the user without a shortcut.
   */
  async function applyShortcut() {
    if (!isDesktop) return
    shortcutError.value = null
    try {
      shortcut.value = await api.setGlobalShortcut(shortcut.value)
    } catch (err) {
      const first = errorMessage(err)
      if (shortcut.value !== DEFAULT_SHORTCUT) {
        try {
          shortcut.value = await api.setGlobalShortcut(DEFAULT_SHORTCUT)
          shortcutError.value = `${first}，已恢复默认快捷键`
          return
        } catch {
          /* fall through */
        }
      }
      shortcutError.value = first
      console.warn('[shortcut] apply failed', err)
    }
  }

  /** Applies a new accelerator; the current one stays active on failure. */
  async function changeShortcut(accelerator: string) {
    shortcutError.value = null
    try {
      shortcut.value = await api.setGlobalShortcut(accelerator)
    } catch (err) {
      shortcutError.value = errorMessage(err)
      throw err
    }
  }

  watch([theme, accent, editorFontSize, sidebarWidth, shortcut], () => void persist())

  return {
    theme,
    accent,
    editorFontSize,
    sidebarWidth,
    shortcut,
    shortcutError,
    ready,
    systemDark,
    resolvedTheme,
    hydrate,
    applyShortcut,
    changeShortcut,
  }
})
