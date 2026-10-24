import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import NotePreview from './NotePreview.vue'

const copyText = vi.fn<(value: string) => Promise<void>>(async () => {})

vi.mock('@/services/clipboard', () => ({ copyText: (value: string) => copyText(value) }))

function mountPreview(content: string) {
  return mount(NotePreview, { props: { content } })
}

describe('NotePreview', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    copyText.mockClear()
  })

  it('renders an empty note without errors', () => {
    const wrapper = mountPreview('')
    expect(wrapper.find('article').exists()).toBe(true)
    expect(wrapper.text()).toBe('')
  })

  it('renders prose as HTML and fences as copyable blocks', async () => {
    const wrapper = mountPreview('# 标题\n\n```bash\necho hi\n```\n\n结尾')
    await flushPromises()
    expect(wrapper.find('h1').text()).toBe('标题')
    const blocks = wrapper.findAll('.code-block')
    expect(blocks).toHaveLength(1)
    expect(blocks[0].find('.lang').text()).toBe('bash')
    expect(blocks[0].text()).toContain('echo hi')
    expect(wrapper.text()).toContain('结尾')
  })

  it('keeps an unterminated fence copyable', async () => {
    const wrapper = mountPreview('```js\nconst a = 1')
    await flushPromises()
    const block = wrapper.find('.code-block')
    expect(block.exists()).toBe(true)
    await block.find('button').trigger('click')
    await flushPromises()
    expect(copyText).toHaveBeenCalledWith('const a = 1')
  })

  it('does not execute inline HTML from a note', () => {
    const wrapper = mountPreview('<script>window.__xss = 1</script>')
    expect(wrapper.find('script').exists()).toBe(false)
    expect((window as unknown as { __xss?: number }).__xss).toBeUndefined()
    expect(wrapper.text()).toContain('<script>')
  })

  it('does not build an anchor for a javascript: link', () => {
    const wrapper = mountPreview('[x](javascript:alert(1))')
    expect(wrapper.find('a').exists()).toBe(false)
  })

  it('opens http links through the system browser instead of navigating', async () => {
    const wrapper = mountPreview('[docs](https://example.com)')
    const anchor = wrapper.find('a')
    expect(anchor.attributes('href')).toBe('https://example.com')
    const open = vi.spyOn(window, 'open').mockImplementation(() => null)
    const event = new MouseEvent('click', { bubbles: true, cancelable: true })
    anchor.element.dispatchEvent(event)
    await flushPromises()
    expect(event.defaultPrevented).toBe(true)
    expect(open).toHaveBeenCalledWith('https://example.com', '_blank', 'noopener')
    open.mockRestore()
  })
})

describe('NotePreview highlighting', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('keeps the rest of the note when one block cannot be highlighted', async () => {
    const wrapper = mountPreview('前文\n\n```nosuchlang\nplain text\n```\n\n后文')
    await flushPromises()
    expect(wrapper.text()).toContain('前文')
    expect(wrapper.text()).toContain('后文')
    expect(wrapper.find('.code-block').exists()).toBe(true)
  })
})
