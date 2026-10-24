import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import TagList from './TagList.vue'
import { useNotesStore } from '@/stores/notes'
import type { TagWithCount } from '@/services/types'

const listTags = vi.fn()
const createTag = vi.fn()
const renameTag = vi.fn()
const deleteTag = vi.fn()
const setTagColor = vi.fn()
const listNotes = vi.fn()

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  CommandError: class extends Error {},
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  api: {
    listTags: (...args: unknown[]) => listTags(...(args as [])),
    createTag: (...args: unknown[]) => createTag(...(args as [])),
    renameTag: (...args: unknown[]) => renameTag(...(args as [])),
    deleteTag: (...args: unknown[]) => deleteTag(...(args as [])),
    setTagColor: (...args: unknown[]) => setTagColor(...(args as [])),
    listNotes: (...args: unknown[]) => listNotes(...(args as [])),
  },
}))

const TAGS: TagWithCount[] = [
  { id: 'tag-1', name: 'db', color: null, noteCount: 3 },
  { id: 'tag-2', name: 'work', color: '#30d158', noteCount: 1 },
]

function setup(tags: TagWithCount[] = TAGS) {
  const store = useNotesStore()
  store.tags = tags
  listTags.mockResolvedValue(tags)
  listNotes.mockResolvedValue({ items: [], nextCursor: null })
  return { wrapper: mount(TagList), store }
}

beforeEach(() => {
  setActivePinia(createPinia())
  listTags.mockReset()
  createTag.mockReset()
  renameTag.mockReset()
  deleteTag.mockReset()
  setTagColor.mockReset()
  listNotes.mockReset()
})

describe('TagList rows', () => {
  it('renders a row per tag with its live count', () => {
    const { wrapper } = setup()
    const names = wrapper.findAll('.row .tag .name').map((node) => node.text())
    expect(names).toEqual(['db', 'work'])
    expect(wrapper.findAll('.row .count').map((node) => node.text())).toEqual(['3', '1'])
  })

  it('clicking the name area only filters — it never starts editing', async () => {
    const { wrapper, store } = setup()

    await wrapper.findAll('.row .tag')[0].trigger('click')

    expect(store.activeTagIds).toEqual(['tag-1'])
    expect(wrapper.find('input').exists()).toBe(false)
    expect(wrapper.find('[data-testid="confirm-delete-tag"]').exists()).toBe(false)
  })

  it('marks at most one tag as active — the filter is single-select', async () => {
    const { wrapper, store } = setup()

    await wrapper.findAll('.row .tag')[0].trigger('click')
    await wrapper.findAll('.row .tag')[1].trigger('click')

    const active = wrapper.findAll('.row .tag').filter((node) => node.classes().includes('active'))
    expect(active).toHaveLength(1)
    expect(active[0].find('.name').text()).toBe('work')
    expect(store.activeTagIds).toEqual(['tag-2'])
  })

  it('shows an empty state instead of a bare list', () => {
    const { wrapper } = setup([])
    expect(wrapper.find('.empty').text()).toBe('还没有标签')
  })
})

describe('TagList rename', () => {
  it('renames inline and closes the editor', async () => {
    const { wrapper } = setup()
    renameTag.mockResolvedValue({ id: 'tag-1', name: 'database', color: null })
    listTags.mockResolvedValue([{ ...TAGS[0], name: 'database' }, TAGS[1]])

    await wrapper.findAll('[data-testid="rename-tag"]')[0].trigger('click')
    const input = wrapper.find('input')
    expect((input.element as HTMLInputElement).value).toBe('db')

    await input.setValue('database')
    await input.trigger('keydown.enter')
    await flushPromises()

    expect(renameTag).toHaveBeenCalledWith('tag-1', 'database')
    expect(wrapper.find('input').exists()).toBe(false)
  })

  it('does not call the command on Escape', async () => {
    const { wrapper } = setup()

    await wrapper.findAll('[data-testid="rename-tag"]')[0].trigger('click')
    await wrapper.find('input').setValue('database')
    await wrapper.find('input').trigger('keydown.esc')
    await flushPromises()

    expect(renameTag).not.toHaveBeenCalled()
    expect(wrapper.find('input').exists()).toBe(false)
  })

  it('keeps the editor open and shows the conflict', async () => {
    const { wrapper } = setup()
    renameTag.mockRejectedValue(new Error('tag already exists: rust'))

    await wrapper.findAll('[data-testid="rename-tag"]')[0].trigger('click')
    await wrapper.find('input').setValue('rust')
    await wrapper.find('input').trigger('keydown.enter')
    await flushPromises()

    expect(wrapper.find('[role="alert"]').text()).toContain('already exists')
    expect(wrapper.find('input').exists()).toBe(true)
  })

  it('refuses a blank name without calling the command', async () => {
    const { wrapper } = setup()

    await wrapper.findAll('[data-testid="rename-tag"]')[0].trigger('click')
    await wrapper.find('input').setValue('   ')
    await wrapper.find('input').trigger('keydown.enter')
    await flushPromises()

    expect(renameTag).not.toHaveBeenCalled()
    expect(wrapper.find('[role="alert"]').text()).toBe('标签名称不能为空')
  })
})

describe('TagList delete', () => {
  it('confirms inline with the affected note count and can be cancelled', async () => {
    const { wrapper } = setup()

    await wrapper.findAll('[data-testid="delete-tag"]')[0].trigger('click')
    expect(wrapper.find('[data-testid="confirm-delete-tag"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('将影响 3 条笔记')

    await wrapper.find('[data-testid="cancel-delete-tag"]').trigger('click')
    expect(deleteTag).not.toHaveBeenCalled()
    expect(wrapper.find('[data-testid="delete-tag"]').exists()).toBe(true)
  })

  it('deletes once on confirm, without touching notes', async () => {
    const { wrapper, store } = setup()
    deleteTag.mockResolvedValue(undefined)
    listTags.mockResolvedValue([TAGS[1]])

    await wrapper.findAll('[data-testid="delete-tag"]')[0].trigger('click')
    await wrapper.find('[data-testid="confirm-delete-tag"]').trigger('click')
    await flushPromises()

    expect(deleteTag).toHaveBeenCalledTimes(1)
    expect(deleteTag).toHaveBeenCalledWith('tag-1')
    expect(store.tags.map((tag) => tag.id)).toEqual(['tag-2'])
  })
})

describe('TagList colour palette', () => {
  it('offers eight swatches and stores the picked one', async () => {
    const { wrapper } = setup()
    setTagColor.mockResolvedValue({ id: 'tag-1', name: 'db', color: '#bf5af2' })
    listTags.mockResolvedValue([{ ...TAGS[0], color: '#bf5af2' }, TAGS[1]])

    await wrapper.findAll('[data-testid="tag-color"]')[0].trigger('click')
    expect(wrapper.find('.palette').findAll('.swatch')).toHaveLength(8)

    await wrapper.find('[data-testid="swatch-#bf5af2"]').trigger('click')
    await flushPromises()

    expect(setTagColor).toHaveBeenCalledWith('tag-1', '#bf5af2')
    expect(wrapper.find('.palette').exists()).toBe(false)
  })
})

describe('TagList create', () => {
  it('creates a tag from the header plus button', async () => {
    const { wrapper } = setup()
    createTag.mockResolvedValue({ id: 'tag-3', name: 'idea', color: null })
    listTags.mockResolvedValue([...TAGS, { id: 'tag-3', name: 'idea', color: null, noteCount: 0 }])

    await wrapper.find('.add').trigger('click')
    await wrapper.find('input').setValue('idea')
    await wrapper.find('input').trigger('keydown.enter')
    await flushPromises()

    expect(createTag).toHaveBeenCalledWith('idea')
    expect(wrapper.find('input').exists()).toBe(false)
  })
})
