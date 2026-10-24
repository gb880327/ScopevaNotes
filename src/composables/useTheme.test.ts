import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { useTheme } from './useTheme'
import { useSettingsStore } from '@/stores/settings'

type Listener = (event: { matches: boolean }) => void

let listeners: Listener[] = []
let matches = false

function mockMatchMedia() {
  window.matchMedia = ((query: string) => ({
    media: query,
    get matches() {
      return matches
    },
    onchange: null,
    addEventListener: (_event: string, listener: Listener) => listeners.push(listener),
    removeEventListener: (_event: string, listener: Listener) => {
      listeners = listeners.filter((item) => item !== listener)
    },
    addListener: (listener: Listener) => listeners.push(listener),
    removeListener: (listener: Listener) => {
      listeners = listeners.filter((item) => item !== listener)
    },
    dispatchEvent: () => true,
  })) as unknown as typeof window.matchMedia
}

function setSystemDark(value: boolean) {
  matches = value
  listeners.forEach((listener) => listener({ matches: value }))
}

const root = document.documentElement

beforeEach(() => {
  listeners = []
  matches = false
  mockMatchMedia()
  setActivePinia(createPinia())
  localStorage.clear()
  delete root.dataset.theme
  delete root.dataset.accent
})

afterEach(() => {
  vi.restoreAllMocks()
  delete root.dataset.theme
  delete root.dataset.accent
})

describe('useTheme', () => {
  it('applies the forced light theme even when the system is dark', async () => {
    matches = true
    const settings = useSettingsStore()
    settings.theme = 'light'
    useTheme()
    await nextTick()
    expect(settings.systemDark).toBe(true)
    expect(settings.resolvedTheme).toBe('light')
    expect(root.dataset.theme).toBe('light')
  })

  it('applies the forced dark theme even when the system is light', async () => {
    const settings = useSettingsStore()
    settings.theme = 'dark'
    useTheme()
    await nextTick()
    expect(root.dataset.theme).toBe('dark')
  })

  it('follows the system appearance when the mode is system', async () => {
    const settings = useSettingsStore()
    settings.theme = 'system'
    useTheme()
    await nextTick()
    expect(root.dataset.theme).toBe('light')

    setSystemDark(true)
    await nextTick()
    expect(settings.resolvedTheme).toBe('dark')
    expect(root.dataset.theme).toBe('dark')

    setSystemDark(false)
    await nextTick()
    expect(root.dataset.theme).toBe('light')
  })

  it('reacts to a theme change from the settings dialog without a restart', async () => {
    const settings = useSettingsStore()
    settings.theme = 'system'
    useTheme()
    await nextTick()
    settings.theme = 'dark'
    await nextTick()
    expect(root.dataset.theme).toBe('dark')
  })

  it('mirrors the accent and editor font size onto the root element', async () => {
    const settings = useSettingsStore()
    settings.accent = 'orange'
    settings.editorFontSize = 20
    useTheme()
    await nextTick()
    expect(root.dataset.accent).toBe('orange')
    expect(root.style.getPropertyValue('--editor-font-size')).toBe('20px')
  })

  it('stops listening to the system after cleanup', async () => {
    const settings = useSettingsStore()
    settings.theme = 'system'
    const dispose = useTheme()
    await nextTick()
    dispose()
    setSystemDark(true)
    await nextTick()
    expect(settings.systemDark).toBe(false)
  })
})
