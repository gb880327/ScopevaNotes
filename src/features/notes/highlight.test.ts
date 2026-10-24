import { describe, expect, it } from 'vitest'
import { highlightToTokens, resolveLanguage, tokenStyle } from './highlight'

describe('resolveLanguage', () => {
  it.each([
    ['bash', 'bash'],
    ['TypeScript', 'typescript'],
    ['  sql  ', 'sql'],
    ['ts', 'typescript'],
    ['sh', 'shellscript'],
    ['yml', 'yaml'],
    ['c++', 'cpp'],
  ])('resolves %s', (tag, expected) => {
    expect(resolveLanguage(tag)).toBe(expected)
  })

  it.each(['', '  ', 'nosuchlang', 'text'])('returns null for %s', (tag) => {
    expect(resolveLanguage(tag)).toBeNull()
  })
})

describe('tokenStyle', () => {
  it('maps colours and font styles', () => {
    expect(tokenStyle({ content: 'x', color: '#fff' })).toEqual({ color: '#fff' })
    expect(tokenStyle({ content: 'x', fontStyle: 1 })).toEqual({ fontStyle: 'italic' })
    expect(tokenStyle({ content: 'x', color: '#000', fontStyle: 2 | 4 })).toEqual({
      color: '#000',
      fontWeight: '600',
      textDecoration: 'underline',
    })
    expect(tokenStyle({ content: 'x' })).toEqual({})
  })
})

describe('highlightToTokens', () => {
  it('returns coloured tokens for a supported language', async () => {
    const tokens = await highlightToTokens('const a = 1', 'ts', 'dark')
    expect(tokens).not.toBeNull()
    expect(tokens!.flat().map((token) => token.content).join('')).toBe('const a = 1')
    expect(tokens!.flat().some((token) => token.color)).toBe(true)
  })

  it('returns null for an unsupported language instead of throwing', async () => {
    await expect(highlightToTokens('x', 'nosuchlang', 'light')).resolves.toBeNull()
  })

  it('returns null for blank code', async () => {
    await expect(highlightToTokens('   ', 'bash', 'light')).resolves.toBeNull()
  })

  it('skips oversized blocks', async () => {
    const huge = 'a'.repeat(20_001)
    await expect(highlightToTokens(huge, 'bash', 'light')).resolves.toBeNull()
  })
})
