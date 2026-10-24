import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useNotesStore, TagAttachError } from './notes'
import type { NoteDetail, NoteSummary, TagWithCount } from '@/services/types'

const listNotes = vi.fn()
const updateNote = vi.fn()
const listTags = vi.fn()
const createTag = vi.fn()
const renameTag = vi.fn()
const deleteTag = vi.fn()
const setTagColor = vi.fn()

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  CommandError: class extends Error {},
  errorMessage: (error: unknown) => String(error),
  api: {
    listNotes: (...args: unknown[]) => listNotes(...(args as [])),
    listTags: (...args: unknown[]) => listTags(...(args as [])),
    getNote: async () => null,
    updateNote: (...args: unknown[]) => updateNote(...(args as [])),
    listDeletedNotes: async () => [],
    createTag: (...args: unknown[]) => createTag(...(args as [])),
    renameTag: (...args: unknown[]) => renameTag(...(args as [])),
    deleteTag: (...args: unknown[]) => deleteTag(...(args as [])),
    setTagColor: (...args: unknown[]) => setTagColor(...(args as [])),
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
  listTags.mockReset()
  createTag.mockReset()
  renameTag.mockReset()
  deleteTag.mockReset()
  setTagColor.mockReset()
  listNotes.mockResolvedValue({ items: [], nextCursor: null })
  listTags.mockResolvedValue([])
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

function tagWithCount(id: string, name: string, noteCount = 1): TagWithCount {
  return { id, name, color: null, noteCount }
}

function noteWithTags(tags: NoteDetail['tags']): NoteDetail {
  return {
    id: 'note-1',
    title: '标题',
    content: '正文',
    isFavorite: false,
    deletedAt: null,
    createdAt: 1,
    updatedAt: 1,
    tags,
  }
}

describe('notes store tag management', () => {
  it('renames the tag in the sidebar and on the open note, then re-queries', async () => {
    const store = useNotesStore()
    store.tags = [tagWithCount('tag-1', 'db', 3)]
    store.selected = noteWithTags([{ id: 'tag-1', name: 'db', color: null }])
    renameTag.mockResolvedValue({ id: 'tag-1', name: 'database', color: null })
    listTags.mockResolvedValue([tagWithCount('tag-1', 'database', 3)])

    await store.renameTag('tag-1', 'database')

    expect(renameTag).toHaveBeenCalledWith('tag-1', 'database')
    expect(store.tags.map((tag) => tag.name)).toEqual(['database'])
    expect(store.selected?.tags[0].name).toBe('database')
    // Tag names live in the search projection, so the list is re-queried.
    expect(listNotes).toHaveBeenCalled()
  })

  it('keeps the sidebar untouched when a rename is rejected', async () => {
    const store = useNotesStore()
    store.tags = [tagWithCount('tag-1', 'db', 3)]
    renameTag.mockRejectedValue(new Error('tag already exists: rust'))

    await expect(store.renameTag('tag-1', 'rust')).rejects.toThrow('already exists')

    expect(store.tags.map((tag) => tag.name)).toEqual(['db'])
    expect(store.error).toContain('already exists')
  })

  it('drops the tag and its filter on delete, without touching notes', async () => {
    const store = useNotesStore()
    store.tags = [tagWithCount('tag-1', 'db', 3), tagWithCount('tag-2', 'work')]
    store.activeTagIds = ['tag-1']
    store.selected = noteWithTags([{ id: 'tag-1', name: 'db', color: null }])
    deleteTag.mockResolvedValue(undefined)
    listTags.mockResolvedValue([tagWithCount('tag-2', 'work')])

    await store.deleteTag('tag-1')

    expect(store.tags.map((tag) => tag.id)).toEqual(['tag-2'])
    expect(store.activeTagIds).toEqual([])
    expect(store.selected?.tags).toEqual([])
  })

  it('stores a tag colour and reflects it on the open note', async () => {
    const store = useNotesStore()
    store.tags = [tagWithCount('tag-1', 'db')]
    store.selected = noteWithTags([{ id: 'tag-1', name: 'db', color: null }])
    setTagColor.mockResolvedValue({ id: 'tag-1', name: 'db', color: '#0a66c2' })
    listTags.mockResolvedValue([{ ...tagWithCount('tag-1', 'db'), color: '#0a66c2' }])

    await store.setTagColor('tag-1', '#0a66c2')

    expect(setTagColor).toHaveBeenCalledWith('tag-1', '#0a66c2')
    expect(store.tags[0].color).toBe('#0a66c2')
    expect(store.selected?.tags[0].color).toBe('#0a66c2')
  })

  it('attaches a tag through update_note and refreshes the counts', async () => {
    const store = useNotesStore()
    store.tags = [tagWithCount('tag-2', 'work', 1)]
    store.selected = noteWithTags([])
    updateNote.mockImplementation(async (args: { tagIds: string[] }) => ({
      ...noteWithTags([{ id: 'tag-2', name: 'work', color: null }]),
      updatedAt: 2,
      tags: args.tagIds.map((id) => ({ id, name: 'work', color: null })),
    }))

    await store.attachTag('tag-2')

    expect(updateNote).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'note-1', tagIds: ['tag-2'] }),
    )
    expect(store.selected?.tags.map((tag) => tag.id)).toEqual(['tag-2'])
    expect(listTags).toHaveBeenCalled()
  })

  it('detaches a tag through update_note and refreshes the counts', async () => {
    const store = useNotesStore()
    store.selected = noteWithTags([{ id: 'tag-1', name: 'db', color: null }])
    updateNote.mockResolvedValue({ ...noteWithTags([]), updatedAt: 2 })

    await store.detachTag('tag-1')

    expect(updateNote).toHaveBeenCalledWith(expect.objectContaining({ tagIds: [] }))
    expect(store.selected?.tags).toEqual([])
    expect(listTags).toHaveBeenCalled()
  })

  it('flags a tag that was created but could not be attached', async () => {
    const store = useNotesStore()
    store.selected = noteWithTags([])
    createTag.mockResolvedValue({ id: 'tag-9', name: '新标签', color: null })
    listTags.mockResolvedValue([tagWithCount('tag-9', '新标签', 0)])
    updateNote.mockRejectedValue(new Error('database is locked'))

    const failure = store.createAndAttachTag('新标签')

    await expect(failure).rejects.toBeInstanceOf(TagAttachError)
    await expect(failure).rejects.toMatchObject({ tagCreated: true })
    await expect(failure).rejects.toThrow('已创建')
    // The tag really exists, so the sidebar must show it for a retry.
    expect(store.tags.map((tag) => tag.name)).toEqual(['新标签'])
  })
})
