import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import CodeBlock from './CodeBlock.vue'

const copyText = vi.fn<(value: string) => Promise<void>>(async () => {})

vi.mock('@/services/clipboard', () => ({ copyText: (value: string) => copyText(value) }))

function mountBlock(props: { code: string; lang: string }) {
  return mount(CodeBlock, { props })
}

describe('CodeBlock', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    copyText.mockClear()
  })

  it('highlights a known language with coloured spans', async () => {
    const wrapper = mountBlock({ code: 'echo "hi"', lang: 'bash' })
    await flushPromises()
    await vi.waitFor(() => expect(wrapper.findAll('code span').length).toBeGreaterThan(1))
    const styled = wrapper.findAll('code span').filter((span) => span.attributes('style'))
    expect(styled.length).toBeGreaterThan(0)
    expect(wrapper.text()).toContain('echo')
  })

  it('maps aliases such as ts to typescript', async () => {
    const wrapper = mountBlock({ code: 'const a: number = 1', lang: 'ts' })
    await vi.waitFor(() =>
      expect(wrapper.findAll('code span').length).toBeGreaterThan(1),
    )
  })

  it('renders an unknown language as plain text', async () => {
    const wrapper = mountBlock({ code: 'whatever it is', lang: 'nosuchlang' })
    await flushPromises()
    const spans = wrapper.findAll('code span')
    expect(spans).toHaveLength(1)
    expect(spans[0].text()).toBe('whatever it is')
    expect(spans[0].attributes('style')).toBeUndefined()
    expect(wrapper.find('.lang').text()).toBe('nosuchlang')
  })

  it('renders an unlabelled block as plain text with a text badge', async () => {
    const wrapper = mountBlock({ code: 'plain', lang: '' })
    await flushPromises()
    expect(wrapper.find('.lang').text()).toBe('text')
    expect(wrapper.find('code').text()).toBe('plain')
  })

  it('copies exactly the block source, without fences', async () => {
    const wrapper = mountBlock({ code: 'const a = 1\nconst b = 2', lang: 'typescript' })
    await wrapper.find('button').trigger('click')
    await flushPromises()
    expect(copyText).toHaveBeenCalledWith('const a = 1\nconst b = 2')
  })

  it('switches the button label to 已复制 and back', async () => {
    vi.useFakeTimers()
    const wrapper = mountBlock({ code: 'x', lang: 'text' })
    expect(wrapper.find('button').text()).toBe('复制代码')
    await wrapper.find('button').trigger('click')
    await flushPromises()
    expect(wrapper.find('button').text()).toBe('已复制')
    vi.advanceTimersByTime(1300)
    await flushPromises()
    expect(wrapper.find('button').text()).toBe('复制代码')
    vi.useRealTimers()
  })

  it('reports a failed copy instead of throwing', async () => {
    copyText.mockRejectedValueOnce(new Error('denied'))
    const wrapper = mountBlock({ code: 'x', lang: 'text' })
    await wrapper.find('button').trigger('click')
    await flushPromises()
    expect(wrapper.find('button').text()).toBe('复制失败')
  })
})
