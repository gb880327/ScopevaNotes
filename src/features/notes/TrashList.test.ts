import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import TrashList from './TrashList.vue'
import { useNotesStore } from '@/stores/notes'
import type { NoteSummary } from '@/services/types'

const listDeletedNotes = vi.fn()
const countDeletedNotes = vi.fn()
const purgeNote = vi.fn()
const emptyTrash = vi.fn()
const restoreNote = vi.fn()

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  CommandError: class extends Error {},
  errorMessage: (error: unknown) => String(error),
  api: {
    listNotes: async () => ({ items: [], nextCursor: null }),
    listTags: async () => [],
    listDeletedNotes: (...args: unknown[]) => listDeletedNotes(...(args as [])),
    countDeletedNotes: (...args: unknown[]) => countDeletedNotes(...(args as [])),
    restoreNote: (...args: unknown[]) => restoreNote(...(args as [])),
    purgeNote: (...args: unknown[]) => purgeNote(...(args as [])),
    emptyTrash: (...args: unknown[]) => emptyTrash(...(args as [])),
  },
}))

function dead(id: string, deletedAt: number, title = id): NoteSummary {
  return {
    id,
    title,
    excerpt: '',
    updatedAt: deletedAt,
    isFavorite: false,
    tags: [],
    deletedAt,
  }
}

const DAY = 24 * 60 * 60 * 1000

async function mountTrash(items: NoteSummary[], count = items.length) {
  listDeletedNotes.mockResolvedValue(items)
  countDeletedNotes.mockResolvedValue(count)
  const store = useNotesStore()
  await store.openTrash()
  return { wrapper: mount(TrashList), store }
}

describe('TrashList', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    for (const fn of [listDeletedNotes, countDeletedNotes, purgeNote, emptyTrash, restoreNote]) {
      fn.mockReset()
    }
    purgeNote.mockResolvedValue(undefined)
    emptyTrash.mockResolvedValue(0)
    restoreNote.mockResolvedValue(undefined)
  })

  it('renders one row per deleted note with its own deletion time', async () => {
    const now = Date.now()
    const { wrapper } = await mountTrash([
      dead('note-1', now - DAY, '废稿一'),
      dead('note-2', now - 3 * DAY, '废稿二'),
    ])

    const rows = wrapper.findAll('.item')
    expect(rows).toHaveLength(2)
    expect(rows[0].find('.title').text()).toBe('废稿一')
    const times = wrapper.findAll('.meta').map((meta) => meta.text())
    expect(times[0]).toContain('1 天前')
    expect(times[1]).toContain('3 天前')
    expect(wrapper.findAll('.restore')).toHaveLength(2)
    expect(wrapper.findAll('.purge')).toHaveLength(2)
  })

  it('restores a note without asking for confirmation', async () => {
    const { wrapper, store } = await mountTrash([dead('note-1', Date.now())])
    listDeletedNotes.mockResolvedValue([])
    countDeletedNotes.mockResolvedValue(0)

    await wrapper.find('.restore').trigger('click')
    await wrapper.vm.$nextTick()

    expect(restoreNote).toHaveBeenCalledWith('note-1')
    expect(store.deleted).toEqual([])
  })

  it('confirms a permanent delete inline and sends nothing on cancel', async () => {
    const { wrapper } = await mountTrash([dead('note-1', Date.now(), '废稿一')])

    await wrapper.find('[data-testid="purge-note-1"]').trigger('click')
    const confirm = wrapper.find('[data-testid="purge-confirm"]')
    expect(confirm.exists()).toBe(true)
    expect(confirm.text()).toContain('永久删除后不可恢复')
    expect(wrapper.find('.item .title').text()).toBe('废稿一') // row stays readable
    expect(purgeNote).not.toHaveBeenCalled()

    await wrapper.find('[data-testid="cancel-purge"]').trigger('click')
    expect(wrapper.find('[data-testid="purge-confirm"]').exists()).toBe(false)
    expect(purgeNote).not.toHaveBeenCalled()

    await wrapper.find('[data-testid="purge-note-1"]').trigger('click')
    listDeletedNotes.mockResolvedValue([])
    countDeletedNotes.mockResolvedValue(0)
    await wrapper.find('[data-testid="confirm-purge"]').trigger('click')
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(purgeNote).toHaveBeenCalledTimes(1)
    expect(purgeNote).toHaveBeenCalledWith('note-1')
    expect(wrapper.findAll('.item')).toHaveLength(0)
  })

  it('labels the clear button with the whole-trash count, not the capped listing', async () => {
    const { wrapper } = await mountTrash([dead('note-1', Date.now())], 40)

    const button = wrapper.find('[data-testid="empty-trash"]')
    expect(button.text()).toContain('40')
    expect(wrapper.find('.count').text()).toContain('40')
  })

  it('clears the whole trash once confirmed, and nothing on cancel', async () => {
    const { wrapper } = await mountTrash([dead('note-1', Date.now())], 2)

    await wrapper.find('[data-testid="empty-trash"]').trigger('click')
    expect(wrapper.find('[data-testid="clear-confirm"]').text()).toContain('不可恢复')
    await wrapper.find('[data-testid="cancel-empty-trash"]').trigger('click')
    expect(emptyTrash).not.toHaveBeenCalled()

    await wrapper.find('[data-testid="empty-trash"]').trigger('click')
    listDeletedNotes.mockResolvedValue([])
    countDeletedNotes.mockResolvedValue(0)
    await wrapper.find('[data-testid="confirm-empty-trash"]').trigger('click')
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(emptyTrash).toHaveBeenCalledTimes(1)
    expect(wrapper.find('[data-testid="empty-trash"]').exists()).toBe(false)
    expect(wrapper.find('.hint').text()).toContain('垃圾箱是空的')
  })

  it('shows the empty state instead of an empty list', async () => {
    const { wrapper } = await mountTrash([])

    expect(wrapper.findAll('.item')).toHaveLength(0)
    expect(wrapper.find('.hint').text()).toContain('垃圾箱是空的')
    expect(wrapper.find('[data-testid="empty-trash"]').exists()).toBe(false)
  })

  it('closes an open confirmation with Escape', async () => {
    const { wrapper } = await mountTrash([dead('note-1', Date.now())])

    await wrapper.find('[data-testid="purge-note-1"]').trigger('click')
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('[data-testid="purge-confirm"]').exists()).toBe(false)
  })

  it('keeps the row when the permanent delete fails', async () => {
    const { wrapper, store } = await mountTrash([dead('note-1', Date.now())])

    purgeNote.mockRejectedValue(new Error('database is locked'))
    await wrapper.find('[data-testid="purge-note-1"]').trigger('click')
    await wrapper.find('[data-testid="confirm-purge"]').trigger('click')
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.findAll('.item')).toHaveLength(1)
    expect(store.error).toContain('database is locked')
  })
})
