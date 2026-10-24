import { describe, expect, it } from 'vitest'
import { isSafeUrl, markdown } from './markdown'

describe('isSafeUrl', () => {
  it.each([
    ['https://example.com/a?b=1', true],
    ['http://example.com', true],
    ['mailto:me@example.com', true],
    ['/notes/1', true],
    ['#heading', true],
    ['notes/1', true],
  ])('allows %s', (url, expected) => {
    expect(isSafeUrl(url)).toBe(expected)
  })

  it.each([
    ['javascript:alert(1)', false],
    ['JaVaScRiPt:alert(1)', false],
    ['java\tscript:alert(1)', false],
    ['java\nscript:alert(1)', false],
    ['vbscript:msgbox(1)', false],
    ['data:text/html,<script>alert(1)</script>', false],
    ['file:///etc/passwd', false],
    ['//evil.example.com', false],
    ['', false],
  ])('rejects %s', (url, expected) => {
    expect(isSafeUrl(url)).toBe(expected)
  })
})

describe('markdown rendering safety', () => {
  it('renders inline HTML as text instead of markup', () => {
    const html = markdown.render('<script>alert(1)</script>')
    expect(html).not.toContain('<script')
    expect(html).toContain('&lt;script&gt;')
  })

  it('renders event-handler HTML as text', () => {
    const html = markdown.render('<img src=x onerror="alert(1)">')
    expect(html).not.toContain('<img')
    expect(html).toContain('onerror')
  })

  it('does not create an anchor for a javascript: link', () => {
    const html = markdown.render('[x](javascript:alert(1))')
    expect(html).not.toContain('<a')
    expect(html).not.toContain('href')
  })

  it('does not create an anchor for an obfuscated scheme', () => {
    const html = markdown.render('[x](java\tscript:alert(1))')
    expect(html).not.toContain('<a')
  })

  it('keeps an https link clickable', () => {
    const html = markdown.render('[docs](https://example.com/page)')
    expect(html).toContain('href="https://example.com/page"')
  })

  it('renders a fence without a language as plain code', () => {
    const html = markdown.render('```\necho hi\n```')
    expect(html).toContain('<pre><code>echo hi')
  })

  it('does not blow up on an unclosed fence', () => {
    expect(() => markdown.render('```bash\necho hi')).not.toThrow()
  })
})
