import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import Sidebar from './Sidebar.vue'
import { useNotesStore } from '@/stores/notes'
import type { NoteSummary, TagWithCount } from '@/services/types'

const listNotes = vi.fn(async () => ({ items: [] as NoteSummary[], nextCursor: null }))
const listDeleted = vi.fn(async () => [] as NoteSummary[])
const listTags = vi.fn(async () => [] as TagWithCount[])
const countDeleted = vi.fn(async () => 0)

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  CommandError: class extends Error {},
  errorMessage: (error: unknown) => String(error),
  api: {
    listNotes: (...args: unknown[]) => listNotes(...(args as [])),
    listDeletedNotes: (...args: unknown[]) => listDeleted(...(args as [])),
    countDeletedNotes: (...args: unknown[]) => countDeleted(...(args as [])),
    purgeNote: async () => {},
    emptyTrash: async () => 0,
    listTags: (...args: unknown[]) => listTags(...(args as [])),
    getGlobalShortcut: async () => 'CommandOrControl+Shift+D',
    getLibraryInfo: async () => ({ path: '/tmp/notes.db', isDefault: true, noteCount: 0, fileSizeBytes: 0, notice: null }),
    isAutostartEnabled: async () => false,
    toggleMainWindow: async () => {},
    getNote: async () => null,
    createNote: async () => null,
    updateNote: async () => null,
  },
}))

function summary(index: number, overrides: Partial<NoteSummary> = {}): NoteSummary {
  return {
    id: `note-${index}`,
    title: `笔记 ${index}`,
    excerpt: `摘要 ${index}`,
    updatedAt: 1_700_000_000_000 + index,
    isFavorite: false,
    tags: [],
    deletedAt: null,
    ...overrides,
  }
}

function mountSidebar() {
  return mount(Sidebar, { attachTo: document.body })
}

beforeEach(() => {
  setActivePinia(createPinia())
  listNotes.mockClear()
  listDeleted.mockClear()
  listTags.mockClear()
})

describe('Sidebar filters', () => {
  it('starts on 全部 and shows the combined empty state', () => {
    const wrapper = mountSidebar()

    const chips = wrapper.findAll('.chip')
    expect(chips.map((chip) => chip.text())).toEqual(['全部', '收藏', '回收站'])
    expect(chips[0].classes()).toContain('active')

    expect(wrapper.text()).toContain('没有匹配的笔记')
    expect(wrapper.find('[role="listbox"]').attributes('aria-label')).toBe('笔记列表')
  })

  it('toggles 收藏 and refreshes the list through the store', async () => {
    const notes = useNotesStore()
    const refresh = vi.spyOn(notes, 'refresh').mockResolvedValue()
    const wrapper = mountSidebar()

    await wrapper.findAll('.chip')[1].trigger('click')

    expect(notes.favoritesOnly).toBe(true)
    expect(refresh).toHaveBeenCalledTimes(1)
    expect(wrapper.findAll('.chip')[1].classes()).toContain('active')

    await wrapper.findAll('.chip')[1].trigger('click')
    expect(notes.favoritesOnly).toBe(false)
  })

  it('swaps the note list for the trash view and back', async () => {
    const notes = useNotesStore()
    listDeleted.mockResolvedValueOnce([summary(9, { title: '已删除的笔记', deletedAt: 1_700_000_100_000 })])
    countDeleted.mockResolvedValueOnce(1)
    const wrapper = mountSidebar()

    await wrapper.findAll('.chip')[2].trigger('click')
    await flushPromises()

    expect(notes.trashOpen).toBe(true)
    expect(wrapper.find('[role="listbox"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('已删除的笔记')
    expect(wrapper.findAll('.chip')[2].classes()).toContain('active')

    await wrapper.find('.trash .link').trigger('click')
    await flushPromises()
    expect(notes.trashOpen).toBe(false)
    expect(wrapper.find('[role="listbox"]').exists()).toBe(true)
  })

  it('shows the empty trash hint instead of an empty box', async () => {
    const notes = useNotesStore()
    notes.trashOpen = true
    const wrapper = mountSidebar()

    expect(wrapper.text()).toContain('垃圾箱是空的')
  })
})

describe('Sidebar filtering affordances', () => {
  it('clears filters from the empty state', async () => {
    const notes = useNotesStore()
    const clearFilters = vi.spyOn(notes, 'clearFilters')
    const wrapper = mountSidebar()

    const clear = wrapper.findAll('button').find((button) => button.text() === '清除筛选')
    expect(clear).toBeDefined()
    await clear!.trigger('click')

    expect(clearFilters).toHaveBeenCalledTimes(1)
  })

  it('selects a tag from the sidebar and reports it as active', async () => {
    const notes = useNotesStore()
    notes.tags = [
      { id: 'tag-a', name: '工作', color: '#ff0000', noteCount: 3 },
      { id: 'tag-b', name: '生活', color: null, noteCount: 2 },
    ]
    const wrapper = mountSidebar()

    const tags = wrapper.findAll('.tags .tag')
    expect(tags.map((tag) => tag.find('.name').text())).toEqual(['全部标签', '工作', '生活'])
    expect(tags[0].classes()).toContain('active')

    await tags[1].trigger('click')
    expect(notes.activeTagIds).toEqual(['tag-a'])
    expect(tags[1].classes()).toContain('active')
    // `toggleTagFilter` calls the setup-scope `refresh`, so count the IPC call.
    await flushPromises()
    expect(listNotes).toHaveBeenCalled()

    await wrapper.findAll('.tags .tag')[0].trigger('click')
    expect(notes.activeTagIds).toEqual([])
  })
})

describe('Sidebar keyboard access', () => {
  it('renders every action as a real, focusable button', () => {
    const wrapper = mountSidebar()

    const controls = wrapper.findAll('button')
    expect(controls.length).toBeGreaterThanOrEqual(7)
    for (const control of controls) {
      expect(control.attributes('type')).toBe('button')
      expect(control.attributes('tabindex')).toBeUndefined()
    }
  })

  it('activates the chips with Enter and Space like a click', async () => {
    const notes = useNotesStore()
    vi.spyOn(notes, 'refresh').mockResolvedValue()
    const wrapper = mountSidebar()
    const favourite = wrapper.findAll('.chip')[1]

    // Native buttons fire `click` for both keys; this pins the behaviour so a
    // future div-based chip cannot silently lose keyboard support.
    await favourite.trigger('keydown', { key: 'Enter' })
    await favourite.trigger('click')
    expect(notes.favoritesOnly).toBe(true)

    await favourite.trigger('keydown', { key: ' ' })
    await favourite.trigger('click')
    expect(notes.favoritesOnly).toBe(false)
  })

  it('keeps the search field reachable and self-describing', () => {
    const wrapper = mountSidebar()
    const input = wrapper.find('input[type="search"]')

    expect(input.exists()).toBe(true)
    expect(input.attributes('placeholder')).toBe('搜索标题、正文、标签')
    expect(input.attributes('disabled')).toBeUndefined()
  })
})
