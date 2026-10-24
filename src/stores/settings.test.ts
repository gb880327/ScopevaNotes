import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  api: {},
  errorMessage: (error: unknown) => String(error),
  CommandError: class extends Error {},
}))

import { DEFAULT_SHORTCUT, useSettingsStore } from './settings'

const STORAGE_KEY = 'notes.settings'

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  vi.restoreAllMocks()
})

describe('settings hydration', () => {
  it('starts from defaults when nothing was persisted', async () => {
    const store = useSettingsStore()
    await store.hydrate()

    expect(store.ready).toBe(true)
    expect(store.theme).toBe('system')
    expect(store.accent).toBe('blue')
    expect(store.editorFontSize).toBe(14)
    expect(store.sidebarWidth).toBe(300)
    expect(store.shortcut).toBe(DEFAULT_SHORTCUT)
  })

  it('restores every persisted preference', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        theme: 'dark',
        accent: 'purple',
        editorFontSize: 18,
        sidebarWidth: 380,
        shortcut: 'CommandOrControl+Shift+K',
      }),
    )
    const store = useSettingsStore()
    await store.hydrate()

    expect(store.theme).toBe('dark')
    expect(store.accent).toBe('purple')
    expect(store.editorFontSize).toBe(18)
    expect(store.sidebarWidth).toBe(380)
    expect(store.shortcut).toBe('CommandOrControl+Shift+K')
  })

  it('falls back to defaults instead of blocking startup when the file is corrupt', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    localStorage.setItem(STORAGE_KEY, '{"theme": "dark",')

    const store = useSettingsStore()
    await store.hydrate()

    expect(store.ready).toBe(true)
    expect(store.theme).toBe('system')
    expect(store.editorFontSize).toBe(14)
    expect(warn).toHaveBeenCalled()
  })

  it('ignores a persisted blob of the wrong shape', async () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ theme: 'neon', editorFontSize: 'big' }))

    const store = useSettingsStore()
    await store.hydrate()

    expect(store.editorFontSize).toBe(14)
    // Unknown accent/theme strings are stored as-is; the DOM layer resolves them.
    expect(store.ready).toBe(true)
  })
})
