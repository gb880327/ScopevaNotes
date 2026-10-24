import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useNotesStore } from './notes'
import type { NoteSummary } from '@/services/types'

const listNotes = vi.fn()
const updateNote = vi.fn()

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  CommandError: class extends Error {},
  errorMessage: (error: unknown) => String(error),
  api: {
    listNotes: (...args: unknown[]) => listNotes(...(args as [])),
    listTags: async () => [],
    getNote: async () => null,
    updateNote: (...args: unknown[]) => updateNote(...(args as [])),
    listDeletedNotes: async () => [],
  },
}))

function summary(id: string): NoteSummary {
  return {
    id,
    title: id,
    excerpt: '',
    updatedAt: 1_700_000_000_000,
    isFavorite: false,
    tags: [],
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

afterEach(() => {
  vi.useRealTimers()
})

beforeEach(() => {
  setActivePinia(createPinia())
  listNotes.mockReset()
  updateNote.mockReset()
})

describe('notes store list ordering', () => {
  it('drops a slow response that a newer query has already superseded', async () => {
    const slow = deferred<{ items: NoteSummary[]; nextCursor: null }>()
    const fast = deferred<{ items: NoteSummary[]; nextCursor: null }>()
    listNotes.mockReturnValueOnce(slow.promise).mockReturnValueOnce(fast.promise)

    const store = useNotesStore()
    store.query = 'sq'
    const stale = store.refresh()
    store.query = 'sql'
    const fresh = store.refresh()

    fast.resolve({ items: [summary('note-sql')], nextCursor: null })
    await fresh
    slow.resolve({ items: [summary('note-sq')], nextCursor: null })
    await stale

    expect(store.items.map((item) => item.id)).toEqual(['note-sql'])
    expect(store.loading).toBe(false)
  })

  it('drops a stale error so it cannot mask a newer result', async () => {
    const slow = deferred<never>()
    listNotes
      .mockReturnValueOnce(slow.promise)
      .mockResolvedValueOnce({ items: [summary('note-sql')], nextCursor: null })

    const store = useNotesStore()
    const stale = store.refresh()
    store.query = 'sql'
    await store.refresh()

    slow.resolve(undefined as never)
    await stale

    expect(store.error).toBeNull()
    expect(store.items.map((item) => item.id)).toEqual(['note-sql'])
  })

  it('does not append a page that was in flight when the query changed', async () => {
    const page = deferred<{ items: NoteSummary[]; nextCursor: string | null }>()
    listNotes
      .mockResolvedValueOnce({ items: [summary('note-1')], nextCursor: 'cursor-1' })
      .mockReturnValueOnce(page.promise)
      .mockResolvedValueOnce({ items: [summary('note-9')], nextCursor: null })

    const store = useNotesStore()
    await store.refresh()
    const more = store.loadMore()

    store.query = 'other'
    await store.refresh()
    page.resolve({ items: [summary('note-2')], nextCursor: 'cursor-2' })
    await more

    expect(store.items.map((item) => item.id)).toEqual(['note-9'])
    expect(store.nextCursor).toBeNull()
  })
})

describe('notes store after an external write', () => {
  it('a refresh issued after an import replaces the list and drops the older page', async () => {
    const beforeImport = deferred<{ items: NoteSummary[]; nextCursor: null }>()
    listNotes
      .mockReturnValueOnce(beforeImport.promise)
      .mockResolvedValueOnce({
        items: [summary('imported-1'), summary('imported-2'), summary('note-old')],
        nextCursor: null,
      })

    const store = useNotesStore()
    const stale = store.refresh()

    // The importer finished and the dialog called refresh() again.
    const fresh = store.refresh()
    await fresh
    beforeImport.resolve({ items: [summary('note-old')], nextCursor: null })
    await stale

    expect(store.items.map((item) => item.id)).toEqual([
      'imported-1',
      'imported-2',
      'note-old',
    ])
  })
})

describe('notes store optimistic concurrency', () => {
  it('keeps the buffer typed during a save in flight when the response lands', async () => {
    const inflight = deferred<{
      id: string
      title: string
      content: string
      updatedAt: number
      isFavorite: boolean
      tags: never[]
    }>()
    updateNote.mockReturnValueOnce(inflight.promise)

    const store = useNotesStore()
    store.selected = {
      id: 'note-1',
      title: 'Title',
      content: 'first draft',
      createdAt: 1,
      updatedAt: 1,
      deletedAt: null,
      isFavorite: false,
      tags: [],
    }

    vi.useFakeTimers()
    store.editContent('first draft')
    // Let the 600 ms debounce fire so the request is genuinely in flight.
    await vi.advanceTimersByTimeAsync(600)
    store.editContent('second draft')

    inflight.resolve({
      id: 'note-1',
      title: 'Title',
      content: 'first draft',
      updatedAt: 2,
      isFavorite: false,
      tags: [],
    })
    await store.flush()
    vi.useRealTimers()

    // The response belongs to the older version, so it may only refresh the
    // timestamp — never overwrite text typed while the request was in flight.
    expect(store.selected?.content).toBe('second draft')
    expect(store.selected?.updatedAt).toBe(2)
  })
})
