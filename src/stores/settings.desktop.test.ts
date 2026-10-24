import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const data: Record<string, unknown> = {}
const saves = vi.fn(async () => {})


vi.mock('@tauri-apps/plugin-store', () => ({
  load: async () => ({
    get: async (key: string) => data[key],
    set: async (key: string, value: unknown) => {
      data[key] = value
    },
    save: saves,
  }),
}))

vi.mock('@/services/commands', () => ({
  isDesktop: true,
  api: {},
  errorMessage: (error: unknown) => String(error),
  CommandError: class extends Error {},
}))

import { useSettingsStore } from './settings'

beforeEach(() => {
  setActivePinia(createPinia())
  for (const key of Object.keys(data)) delete data[key]
  saves.mockClear()
})

describe('settings hydration on the desktop store', () => {
  it('reads preferences from the pinned store key', async () => {
    data.appearance = {
      theme: 'dark',
      accent: 'green',
      editorFontSize: 16,
      sidebarWidth: 340,
      shortcut: 'CommandOrControl+Shift+K',
    }

    const store = useSettingsStore()
    await store.hydrate()

    expect(store.theme).toBe('dark')
    expect(store.accent).toBe('green')
    expect(store.editorFontSize).toBe(16)
    expect(store.sidebarWidth).toBe(340)
    expect(store.shortcut).toBe('CommandOrControl+Shift+K')
  })

  it('keeps defaults when the store has no preferences yet', async () => {
    const store = useSettingsStore()
    await store.hydrate()

    expect(store.ready).toBe(true)
    expect(store.theme).toBe('system')
    expect(store.accent).toBe('blue')
  })

})
