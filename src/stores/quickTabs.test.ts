import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { DRAFT_DEBOUNCE_MS, useQuickTabsStore } from './quickTabs'
import type { QuickTab } from '@/services/types'

const listQuickTabs = vi.fn()
const createQuickTab = vi.fn()
const updateQuickTab = vi.fn()
const deleteQuickTab = vi.fn()
const duplicateQuickTab = vi.fn()
const quickTabLimits = vi.fn()
const hideQuickTabs = vi.fn()

class FakeCommandError extends Error {
  readonly kind: string
  constructor(kind: string, message: string) {
    super(message)
    this.kind = kind
  }
}

// The store only uses `api` and `errorMessage`; nothing else from the module
// needs to exist, and a hoisted factory must not reference outer bindings.
vi.mock('@/services/commands', () => ({
  errorMessage: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
  api: {
    listQuickTabs: (...args: unknown[]) => listQuickTabs(...(args as [])),
    createQuickTab: (...args: unknown[]) => createQuickTab(...(args as [])),
    updateQuickTab: (...args: unknown[]) => updateQuickTab(...(args as [])),
    deleteQuickTab: (...args: unknown[]) => deleteQuickTab(...(args as [])),
    duplicateQuickTab: (...args: unknown[]) => duplicateQuickTab(...(args as [])),
    quickTabLimits: (...args: unknown[]) => quickTabLimits(...(args as [])),
    hideQuickTabs: (...args: unknown[]) => hideQuickTabs(...(args as [])),
  },
}))

function tab(id: string, overrides: Partial<QuickTab> = {}): QuickTab {
  return {
    id,
    title: '',
    color: 'yellow',
    content: '',
    sortOrder: 0,
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_000_000,
    ...overrides,
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  vi.clearAllMocks()
  quickTabLimits.mockResolvedValue({ maxTabs: 12, colors: ['yellow', 'red'] })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('quickTabs store', () => {
  it('creates a tab and makes it active', async () => {
    listQuickTabs.mockResolvedValue([tab('a')])
    createQuickTab.mockResolvedValue(tab('b', { color: 'red' }))
    const store = useQuickTabsStore()

    await store.init()
    expect(store.tabs).toHaveLength(1)
    expect(store.activeTabId).toBe('a')

    const created = await store.create()
    expect(createQuickTab).toHaveBeenCalledTimes(1)
    expect(created?.id).toBe('b')
    expect(store.tabs.map((item) => item.id)).toEqual(['a', 'b'])
    expect(store.activeTabId).toBe('b')
  })

  it('surfaces the cap error from Rust instead of guessing locally', async () => {
    listQuickTabs.mockResolvedValue(Array.from({ length: 12 }, (_, index) => tab(`t${index}`)))
    createQuickTab.mockRejectedValue(new FakeCommandError('invalid', '速记最多 12 个'))
    const store = useQuickTabsStore()

    await store.init()
    expect(store.atLimit).toBe(true)

    const created = await store.create()
    expect(created).toBeNull()
    expect(store.lastError).toBe('速记最多 12 个')
    expect(store.tabs).toHaveLength(12)
  })

  it('flushes the pending edit before switching tabs', async () => {
    listQuickTabs.mockResolvedValue([tab('a'), tab('b')])
    updateQuickTab.mockResolvedValue(tab('a', { content: 'hello', updatedAt: 2 }))
    const store = useQuickTabsStore()
    await store.init()

    store.updateContent('hello')
    await store.setActive('b')

    expect(updateQuickTab).toHaveBeenCalledWith({ id: 'a', content: 'hello' })
    expect(store.activeTabId).toBe('b')
    expect(store.saveState).toBe('saved')
  })

  it('deletes only when called, and only after the UI confirmed', async () => {
    listQuickTabs.mockResolvedValue([tab('a'), tab('b')])
    deleteQuickTab.mockResolvedValue(undefined)
    const store = useQuickTabsStore()
    await store.init()

    expect(deleteQuickTab).not.toHaveBeenCalled()
    await store.remove('a')

    expect(deleteQuickTab).toHaveBeenCalledTimes(1)
    expect(deleteQuickTab).toHaveBeenCalledWith('a')
    expect(store.tabs.map((item) => item.id)).toEqual(['b'])
    expect(store.activeTabId).toBe('b')
  })

  it('keeps the draft and reports failure when the write fails', async () => {
    listQuickTabs.mockResolvedValue([tab('a')])
    updateQuickTab.mockRejectedValue(new FakeCommandError('busy', '库被占用'))
    const store = useQuickTabsStore()
    await store.init()

    store.updateContent('未保存的文字')
    await store.flush()

    expect(store.activeTab?.content).toBe('未保存的文字')
    expect(store.saveState).toBe('error')
    expect(store.lastError).toBe('库被占用')
    // The snapshot stays so a crash or reload still recovers the text.
    expect(localStorage.getItem('qt:draft:a')).toContain('未保存的文字')
  })

  it('clears the snapshot once the write succeeds', async () => {
    listQuickTabs.mockResolvedValue([tab('a')])
    updateQuickTab.mockResolvedValue(tab('a', { content: 'saved', updatedAt: 2 }))
    const store = useQuickTabsStore()
    await store.init()

    store.updateContent('saved')
    expect(localStorage.getItem('qt:draft:a')).not.toBeNull()
    await store.flush()

    expect(localStorage.getItem('qt:draft:a')).toBeNull()
    expect(store.saveState).toBe('saved')
  })

  it('restores a snapshot that is newer than the stored row', async () => {
    vi.useFakeTimers()
    updateQuickTab.mockResolvedValue(tab('a', { content: '恢复的草稿', updatedAt: Date.now() }))
    listQuickTabs.mockResolvedValue([tab('a', { updatedAt: 1_000 })])
    localStorage.setItem(
      'qt:draft:a',
      JSON.stringify({ content: '恢复的草稿', savedAt: 5_000 }),
    )
    const store = useQuickTabsStore()

    await store.init()
    expect(store.activeTab?.content).toBe('恢复的草稿')
    expect(store.saveState).toBe('saving')

    // The restored text is re-queued, so it reaches the DB without a keystroke.
    await vi.advanceTimersByTimeAsync(DRAFT_DEBOUNCE_MS)
    await flushPromises()
    expect(updateQuickTab).toHaveBeenCalledWith({ id: 'a', content: '恢复的草稿' })
    expect(localStorage.getItem('qt:draft:a')).toBeNull()
  })

  it('drops snapshots that no longer belong to this library', async () => {
    listQuickTabs.mockResolvedValue([tab('a')])
    localStorage.setItem('qt:draft:gone', JSON.stringify({ content: 'x', savedAt: 1 }))
    localStorage.setItem('qt:draft:a', JSON.stringify({ content: 'old', savedAt: 1 }))
    const store = useQuickTabsStore()

    await store.init()

    expect(store.tabs.map((item) => item.id)).toEqual(['a'])
    expect(localStorage.getItem('qt:draft:gone')).toBeNull()
    // Not newer than the row, so nothing is restored.
    expect(store.activeTab?.content).toBe('')
    expect(localStorage.getItem('qt:draft:a')).toBeNull()
  })

  it('flushes before hiding the panel', async () => {
    listQuickTabs.mockResolvedValue([tab('a')])
    updateQuickTab.mockResolvedValue(tab('a', { content: 'bye', updatedAt: 2 }))
    hideQuickTabs.mockResolvedValue(undefined)
    const store = useQuickTabsStore()
    await store.init()

    store.updateContent('bye')
    await store.hide()

    const saveOrder = updateQuickTab.mock.invocationCallOrder[0]
    const hideOrder = hideQuickTabs.mock.invocationCallOrder[0]
    expect(saveOrder).toBeLessThan(hideOrder)
  })

  it('reloads from the new library and forgets the old drafts', async () => {
    listQuickTabs.mockResolvedValue([tab('a')])
    const store = useQuickTabsStore()
    await store.init()

    store.updateContent('旧库草稿')
    listQuickTabs.mockResolvedValue([tab('new', { content: '新库' })])
    await store.reload()

    expect(store.tabs.map((item) => item.id)).toEqual(['new'])
    expect(store.activeTab?.content).toBe('新库')
    expect(localStorage.getItem('qt:draft:a')).toBeNull()
    expect(updateQuickTab).not.toHaveBeenCalled()
  })
})
