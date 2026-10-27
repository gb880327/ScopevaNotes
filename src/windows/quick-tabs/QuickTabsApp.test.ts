import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import QuickTabsApp from './QuickTabsApp.vue'
import type { QuickTab } from '@/services/types'

const listQuickTabs = vi.fn()
const createQuickTab = vi.fn()
const updateQuickTab = vi.fn()
const deleteQuickTab = vi.fn()
const duplicateQuickTab = vi.fn()
const quickTabLimits = vi.fn()
const hideQuickTabs = vi.fn()

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
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

// The panel borrows the global theme, but pulling the real settings store in
// would drag the whole persistence path into a component test.
vi.mock('@/composables/useTheme', () => ({ useTheme: () => () => {} }))
vi.mock('@/stores/settings', () => ({
  useSettingsStore: () => ({ hydrate: async () => {} }),
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

async function mountPanel(tabs: QuickTab[]) {
  listQuickTabs.mockResolvedValue(tabs)
  const wrapper = mount(QuickTabsApp, { attachTo: document.body })
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
  vi.clearAllMocks()
  quickTabLimits.mockResolvedValue({
    maxTabs: 12,
    colors: ['yellow', 'orange', 'red', 'purple', 'blue', 'cyan', 'green'],
  })
})

describe('速记 panel', () => {
  it('renders one dot per tab and reports the active tab status', async () => {
    const wrapper = await mountPanel([
      tab('a', { title: '甲', content: 'hi' }),
      tab('b', { title: '乙', color: 'blue' }),
      tab('c', { title: '丙', color: 'green' }),
    ])

    const dots = wrapper.findAll('[data-testid^="quick-tab-dot-"]')
    expect(dots).toHaveLength(3)
    expect(dots[0].classes()).toContain('active')
    expect(dots[1].classes()).not.toContain('active')
    expect(wrapper.get('[data-testid="quick-tab-status"]').text()).toContain('2 字符')

    await dots[1].trigger('click')
    await flushPromises()
    expect(wrapper.findAll('[data-testid^="quick-tab-dot-"]')[1].classes()).toContain('active')
    expect(wrapper.get('[data-testid="quick-tab-status"]').text()).toContain('0 字符')
    wrapper.unmount()
  })

  it('puts the caret at the very beginning when the panel opens another tab', async () => {
    const body = 'x'.repeat(2000)
    const wrapper = await mountPanel([
      tab('a', { title: '甲', content: `第一行\n${body}` }),
      tab('b', { title: '乙', color: 'blue', content: `第一行\n${body}` }),
    ])

    const area = wrapper.get('[data-testid="quick-tab-editor"]').element as HTMLTextAreaElement
    // What the browser does to a focused textarea that already has content:
    // the caret goes to the end, i.e. the last line is what the user sees.
    area.setSelectionRange(area.value.length, area.value.length)
    expect(area.selectionStart).toBe(area.value.length)

    await wrapper.get('[data-testid="quick-tab-dot-b"]').trigger('click')
    await flushPromises()

    expect(area.selectionStart).toBe(0)
    expect(area.selectionEnd).toBe(0)
    wrapper.unmount()
  })

  it('selects with a ring and a colour-independent accessible name', async () => {
    const wrapper = await mountPanel([tab('a', { title: '甲', color: 'yellow' })])

    const dot = wrapper.get('[data-testid="quick-tab-dot-a"]')
    expect(dot.classes()).toContain('active')
    // Colour + title, so the bar is usable without seeing colour.
    expect(dot.attributes('aria-label')).toBe('速记 甲（黄）')
    expect(dot.attributes('aria-selected')).toBe('true')
    wrapper.unmount()
  })

  it('disables + with the cap message once the limit is reached', async () => {
    const wrapper = await mountPanel(
      Array.from({ length: 12 }, (_, index) => tab(`t${index}`)),
    )

    expect(wrapper.get('[data-testid="quick-tab-add"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-testid="quick-tab-limit"]').text()).toContain('12')
    wrapper.unmount()
  })

  it('creates a tab from + and switches to it', async () => {
    const wrapper = await mountPanel([tab('a')])
    createQuickTab.mockResolvedValue(tab('b', { color: 'red' }))

    await wrapper.get('[data-testid="quick-tab-add"]').trigger('click')
    await flushPromises()

    expect(createQuickTab).toHaveBeenCalledTimes(1)
    expect(wrapper.findAll('[data-testid^="quick-tab-dot-"]')).toHaveLength(2)
    expect(wrapper.findAll('[data-testid^="quick-tab-dot-"]')[1].classes()).toContain('active')
    wrapper.unmount()
  })

  it('never deletes before the confirmation is accepted', async () => {
    const wrapper = await mountPanel([tab('a'), tab('b', { color: 'blue' })])
    deleteQuickTab.mockResolvedValue(undefined)

    await wrapper.get('[data-testid="quick-tab-more"]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="quick-tab-delete"]').trigger('click')
    await flushPromises()

    expect(deleteQuickTab).not.toHaveBeenCalled()
    expect(wrapper.get('[data-testid="quick-tab-menu"]').text()).toContain('不可恢复')

    await wrapper.get('[data-testid="quick-tab-cancel-delete"]').trigger('click')
    await flushPromises()
    expect(deleteQuickTab).not.toHaveBeenCalled()

    await wrapper.get('[data-testid="quick-tab-delete"]').trigger('click')
    await wrapper.get('[data-testid="quick-tab-confirm-delete"]').trigger('click')
    await flushPromises()

    expect(deleteQuickTab).toHaveBeenCalledTimes(1)
    expect(deleteQuickTab).toHaveBeenCalledWith('a')
    wrapper.unmount()
  })

  it('duplicates into an independent row with a new id', async () => {
    const wrapper = await mountPanel([tab('a', { title: '甲', content: '正文' })])
    duplicateQuickTab.mockResolvedValue(
      tab('copy', { title: '甲 副本', content: '正文', color: 'yellow' }),
    )

    await wrapper.get('[data-testid="quick-tab-more"]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="quick-tab-duplicate"]').trigger('click')
    await flushPromises()

    expect(duplicateQuickTab).toHaveBeenCalledWith('a')
    expect(wrapper.findAll('[data-testid^="quick-tab-dot-"]')).toHaveLength(2)
    expect(wrapper.findAll('[data-testid^="quick-tab-dot-"]')[1].classes()).toContain('active')
    expect(wrapper.get('[data-testid="quick-tab-editor"]').element).toHaveProperty(
      'value',
      '正文',
    )
    wrapper.unmount()
  })

  it('renames and recolours through the menu', async () => {
    const wrapper = await mountPanel([tab('a', { title: '旧标题' })])
    updateQuickTab.mockImplementation(async (args: { id: string }) =>
      tab(args.id, { title: '新标题', color: 'purple' }),
    )

    await wrapper.get('[data-testid="quick-tab-more"]').trigger('click')
    await flushPromises()

    const input = wrapper.get('[data-testid="quick-tab-title-input"]')
    await input.setValue('新标题')
    await input.trigger('blur')
    await flushPromises()
    expect(updateQuickTab).toHaveBeenCalledWith({ id: 'a', title: '新标题' })

    await wrapper.get('[data-testid="quick-tab-color-purple"]').trigger('click')
    await flushPromises()
    expect(updateQuickTab).toHaveBeenCalledWith({ id: 'a', color: 'purple' })
    wrapper.unmount()
  })

  it('closes on Escape and leaves every other key alone', async () => {
    const wrapper = await mountPanel([tab('a')])
    hideQuickTabs.mockResolvedValue(undefined)

    const editor = wrapper.get('[data-testid="quick-tab-editor"]')
    for (const key of ['a', 'ArrowLeft', 'ArrowUp', 'Backspace']) {
      await editor.trigger('keydown', { key })
    }
    await flushPromises()
    expect(hideQuickTabs).not.toHaveBeenCalled()

    await editor.trigger('keydown', { key: 'Escape' })
    await flushPromises()
    expect(hideQuickTabs).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('ignores Escape while an IME composition is in progress', async () => {
    const wrapper = await mountPanel([tab('a')])
    hideQuickTabs.mockResolvedValue(undefined)

    const editor = wrapper.get('[data-testid="quick-tab-editor"]')
    await editor.trigger('compositionstart')
    await editor.trigger('keydown', { key: 'Escape' })
    await flushPromises()
    expect(hideQuickTabs).not.toHaveBeenCalled()

    await editor.trigger('compositionend')
    await editor.trigger('keydown', { key: 'Escape' })
    await flushPromises()
    expect(hideQuickTabs).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('says it is saving, then saved, and shows the character count', async () => {
    const wrapper = await mountPanel([tab('a')])
    updateQuickTab.mockResolvedValue(tab('a', { content: 'abc', updatedAt: 2 }))

    const editor = wrapper.get('[data-testid="quick-tab-editor"]')
    await editor.setValue('abc')
    await flushPromises()
    expect(wrapper.get('[data-testid="quick-tab-status"]').text()).toContain('正在保存…')

    vi.useRealTimers()
    await new Promise((resolve) => setTimeout(resolve, 400))
    await flushPromises()
    expect(wrapper.get('[data-testid="quick-tab-status"]').text()).toContain('已保存')
    expect(wrapper.get('[data-testid="quick-tab-status"]').text()).toContain('3 字符')
    wrapper.unmount()
  })
})
