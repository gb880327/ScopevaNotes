import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import NoteHeader from './NoteHeader.vue'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'
import type { NoteDetail } from '@/services/types'

const copyText = vi.fn<(value: string) => Promise<void>>(async () => {})

vi.mock('@/services/clipboard', () => ({ copyText: (value: string) => copyText(value) }))

const note: NoteDetail = {
  id: 'note-1',
  title: '标题',
  content: '# 标题\n\n正文',
  isFavorite: false,
  deletedAt: null,
  tags: [{ id: 'tag-1', name: '工作', color: null }],
  createdAt: 1_700_000_000_000,
  updatedAt: 1_700_000_000_000,
}

function mountHeader() {
  const notes = useNotesStore()
  const ui = useUiStore()
  notes.selected = note
  return { wrapper: mount(NoteHeader), notes, ui }
}

describe('NoteHeader', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    copyText.mockClear()
  })

  it('switches to edit mode and back, flushing before leaving edit', async () => {
    const { wrapper, notes, ui } = mountHeader()
    const flush = vi.spyOn(notes, 'flush')

    expect(wrapper.find('h1').text()).toBe('标题')
    expect(wrapper.find('input.title-input').exists()).toBe(false)

    await wrapper.findAll('button').at(2)!.trigger('click')
    expect(ui.viewMode).toBe('edit')
    expect(wrapper.find('input.title-input').exists()).toBe(true)
    expect(wrapper.find('h1').exists()).toBe(false)

    await wrapper.findAll('button').at(2)!.trigger('click')
    await flushPromises()
    expect(flush).toHaveBeenCalled()
    expect(ui.viewMode).toBe('read')
  })

  it('shows 无标题 for an empty title', () => {
    const { wrapper, notes } = mountHeader()
    notes.selected = { ...note, title: '' }
    return wrapper.vm.$nextTick().then(() => {
      expect(wrapper.find('h1').text()).toBe('无标题')
    })
  })

  it('copies the raw markdown of the whole note', async () => {
    const { wrapper } = mountHeader()
    const copyAll = wrapper.findAll('button').at(1)!
    expect(copyAll.text()).toBe('复制全文')
    await copyAll.trigger('click')
    await flushPromises()
    expect(copyText).toHaveBeenCalledWith(note.content)
    expect(copyAll.text()).toBe('已复制')
  })

  it('toggles the favorite star label', async () => {
    const { wrapper, notes } = mountHeader()
    const star = wrapper.findAll('button').at(0)!
    expect(star.text()).toBe('☆')
    expect(star.attributes('title')).toBe('收藏')
    // A failed IPC must not break the header: the store swallows and reports.
    vi.spyOn(notes, 'toggleFavorite').mockResolvedValue(undefined)
    await star.trigger('click')
    expect(notes.toggleFavorite).toHaveBeenCalled()
  })

  it('renders the note tags', () => {
    const { wrapper } = mountHeader()
    expect(wrapper.find('.tag').text()).toBe('工作')
  })
})
