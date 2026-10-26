import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { api, errorMessage } from '@/services/commands'
import { createAutosaveQueue } from '@/composables/useAutosave'
import type { QuickTab } from '@/services/types'

export type SaveState = 'idle' | 'saving' | 'saved' | 'error'

/** Debounce for the DB write (design D7); the localStorage snapshot is written
 * on every keystroke batch, not on this timer. */
export const DRAFT_DEBOUNCE_MS = 300

export const DEFAULT_COLORS = [
  'yellow',
  'orange',
  'red',
  'purple',
  'blue',
  'cyan',
  'green',
] as const

const MAX_TABS_FALLBACK = 12
const DRAFT_PREFIX = 'qt:draft:'

interface DraftSnapshot {
  content: string
  savedAt: number
}

function readSnapshot(id: string): DraftSnapshot | null {
  try {
    const raw = localStorage.getItem(DRAFT_PREFIX + id)
    if (!raw) return null
    const parsed = JSON.parse(raw) as DraftSnapshot
    if (typeof parsed?.content !== 'string' || typeof parsed?.savedAt !== 'number') return null
    return parsed
  } catch {
    return null
  }
}

function writeSnapshot(id: string, content: string) {
  try {
    localStorage.setItem(
      DRAFT_PREFIX + id,
      JSON.stringify({ content, savedAt: Date.now() } satisfies DraftSnapshot),
    )
  } catch {
    // A full quota must never break typing; the DB write is still pending.
  }
}

function clearSnapshot(id: string) {
  try {
    localStorage.removeItem(DRAFT_PREFIX + id)
  } catch {
    // ignore
  }
}

function snapshotIds(): string[] {
  const ids: string[] = []
  try {
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index)
      if (key?.startsWith(DRAFT_PREFIX)) ids.push(key.slice(DRAFT_PREFIX.length))
    }
  } catch {
    // ignore
  }
  return ids
}

/**
 * 速记 panel state. One rule shapes everything here: the in-memory `content` is
 * the only truth, the DB is a copy and `localStorage` is a safety net. A failed
 * write therefore keeps the draft instead of reloading the old text.
 */
export const useQuickTabsStore = defineStore('quickTabs', () => {
  const tabs = ref<QuickTab[]>([])
  const activeTabId = ref<string | null>(null)
  const loading = ref(false)
  const saveState = ref<SaveState>('idle')
  const lastError = ref<string | null>(null)
  const maxTabs = ref(MAX_TABS_FALLBACK)
  const colors = ref<string[]>([...DEFAULT_COLORS])

  const atLimit = computed(() => tabs.value.length >= maxTabs.value)
  const activeTab = computed(
    () => tabs.value.find((tab) => tab.id === activeTabId.value) ?? null,
  )

  /** id → content that still has to reach the DB. Survives a failed flush. */
  const pending = new Map<string, string>()

  const queue = createAutosaveQueue(async () => {
    for (const id of [...pending.keys()]) {
      const content = pending.get(id)
      if (content === undefined) continue
      try {
        const saved = await api.updateQuickTab({ id, content })
        const index = tabs.value.findIndex((tab) => tab.id === id)
        if (index >= 0) {
          tabs.value[index] = {
            ...tabs.value[index],
            content: saved.content,
            updatedAt: saved.updatedAt,
          }
        }
        // Typing during the write re-arms `pending`; only then keep the snapshot.
        if (pending.get(id) === content) {
          pending.delete(id)
          clearSnapshot(id)
        }
      } catch (err) {
        saveState.value = 'error'
        lastError.value = errorMessage(err)
        throw err
      }
    }
    saveState.value = 'saved'
    lastError.value = null
  }, DRAFT_DEBOUNCE_MS)

  function adopt(list: QuickTab[]) {
    tabs.value = list
    if (!list.some((tab) => tab.id === activeTabId.value)) {
      activeTabId.value = list[0]?.id ?? null
    }
  }

  /**
   * Snapshots that belong to another library (a different UUID never collides)
   * are dropped; snapshots newer than the DB row are restored and re-queued.
   */
  function restoreDrafts(list: QuickTab[]) {
    const known = new Map(list.map((tab) => [tab.id, tab]))
    for (const id of snapshotIds()) {
      const snapshot = readSnapshot(id)
      const tab = known.get(id)
      if (!snapshot || !tab) {
        clearSnapshot(id)
        continue
      }
      if (snapshot.savedAt > tab.updatedAt) {
        const target = tabs.value.find((candidate) => candidate.id === id)
        if (!target) continue
        target.content = snapshot.content
        pending.set(id, snapshot.content)
        saveState.value = 'saving'
        queue.schedule()
      } else {
        clearSnapshot(id)
      }
    }
  }

  async function init() {
    loading.value = true
    try {
      const [list, limits] = await Promise.all([
        api.listQuickTabs(),
        api.quickTabLimits().catch(() => null),
      ])
      if (limits) {
        maxTabs.value = limits.maxTabs
        if (limits.colors.length > 0) colors.value = limits.colors
      }
      adopt(list)
      restoreDrafts(list)
      lastError.value = null
    } catch (err) {
      lastError.value = errorMessage(err)
    } finally {
      loading.value = false
    }
  }

  /** `ui://library-changed` (design D13): the panel must not write into the
   * previous library, and its drafts died with that library. */
  async function reload() {
    queue.discard()
    pending.clear()
    saveState.value = 'idle'
    await init()
  }

  function updateContent(content: string) {
    const tab = activeTab.value
    if (!tab) return
    tab.content = content
    pending.set(tab.id, content)
    writeSnapshot(tab.id, content)
    saveState.value = 'saving'
    queue.schedule()
  }

  async function flush() {
    await queue.flush()
  }

  async function setActive(id: string) {
    if (id === activeTabId.value) return
    // Never let a pending edit land in the newly selected tab (D10).
    await flush()
    activeTabId.value = id
  }

  async function create() {
    await flush()
    try {
      const tab = await api.createQuickTab({})
      tabs.value.push(tab)
      activeTabId.value = tab.id
      lastError.value = null
      saveState.value = 'idle'
      return tab
    } catch (err) {
      // The cap is a Rust-side rule; the UI only shows the message (D8).
      lastError.value = errorMessage(err)
      saveState.value = 'error'
      return null
    }
  }

  async function rename(id: string, title: string) {
    const tab = tabs.value.find((candidate) => candidate.id === id)
    if (!tab) return
    const previous = tab.title
    tab.title = title
    try {
      const saved = await api.updateQuickTab({ id, title })
      tab.title = saved.title
      lastError.value = null
    } catch (err) {
      tab.title = previous
      lastError.value = errorMessage(err)
    }
  }

  async function setColor(id: string, color: string) {
    const tab = tabs.value.find((candidate) => candidate.id === id)
    if (!tab) return
    const previous = tab.color
    tab.color = color
    try {
      const saved = await api.updateQuickTab({ id, color })
      tab.color = saved.color
      lastError.value = null
    } catch (err) {
      tab.color = previous
      lastError.value = errorMessage(err)
    }
  }

  async function duplicate(id: string) {
    await flush()
    try {
      const copy = await api.duplicateQuickTab(id)
      tabs.value.push(copy)
      activeTabId.value = copy.id
      lastError.value = null
      return copy
    } catch (err) {
      lastError.value = errorMessage(err)
      return null
    }
  }

  /** Called only after the UI confirmed — deletion has no undo (spec). */
  async function remove(id: string) {
    const index = tabs.value.findIndex((tab) => tab.id === id)
    if (index < 0) return
    await flush()
    const removed = tabs.value[index]
    tabs.value.splice(index, 1)
    if (activeTabId.value === id) {
      const next = tabs.value[Math.min(index, tabs.value.length - 1)]
      activeTabId.value = next?.id ?? null
    }
    try {
      await api.deleteQuickTab(id)
      pending.delete(id)
      clearSnapshot(id)
      lastError.value = null
    } catch (err) {
      tabs.value.splice(index, 0, removed)
      if (activeTabId.value !== removed.id && !tabs.value.some((t) => t.id === activeTabId.value)) {
        activeTabId.value = removed.id
      }
      lastError.value = errorMessage(err)
    }
  }

  /** Esc / blur / quit: save first, then let Rust hide the panel. */
  async function hide() {
    await flush()
    try {
      await api.hideQuickTabs()
    } catch (err) {
      lastError.value = errorMessage(err)
    }
  }

  return {
    tabs,
    activeTabId,
    activeTab,
    loading,
    saveState,
    lastError,
    maxTabs,
    colors,
    atLimit,
    init,
    reload,
    updateContent,
    flush,
    setActive,
    create,
    rename,
    setColor,
    duplicate,
    remove,
    hide,
  }
})
