import { describe, expect, it } from 'vitest'
import { splitFences } from './fences'

describe('splitFences', () => {
  it('returns prose when there is no fence', () => {
    expect(splitFences('# Title\n\nbody')).toEqual([
      { type: 'markdown', value: '# Title\n\nbody' },
    ])
  })

  it('separates a fenced block and keeps the language', () => {
    const segments = splitFences('before\n```ts\nconst a = 1\n```\nafter')
    expect(segments).toHaveLength(3)
    expect(segments[1]).toEqual({ type: 'code', lang: 'ts', code: 'const a = 1' })
  })

  it('does not include fence markers or trailing newline in code', () => {
    const [code] = splitFences('```bash\necho hi\n\n```')
    expect(code).toEqual({ type: 'code', lang: 'bash', code: 'echo hi\n' })
  })

  it('keeps an unterminated fence as a code block to the end', () => {
    expect(splitFences('```js\nconst a = 1')).toEqual([
      { type: 'code', lang: 'js', code: 'const a = 1' },
    ])
  })

  it('supports tilde fences', () => {
    const segments = splitFences('~~~sql\nselect 1;\n~~~')
    expect(segments).toEqual([{ type: 'code', lang: 'sql', code: 'select 1;' }])
  })

  it('closes only on a fence at least as long as the opening one', () => {
    const input = '````md\n```bash\necho hi\n```\n````'
    expect(splitFences(input)).toEqual([
      { type: 'code', lang: 'md', code: '```bash\necho hi\n```' },
    ])
  })

  it('accepts a longer closing fence', () => {
    expect(splitFences('```\nbody\n`````')).toEqual([
      { type: 'code', lang: '', code: 'body' },
    ])
  })

  it('does not open a fence from backticks inside the info string', () => {
    const input = '```a`b\nnot code'
    expect(splitFences(input)).toEqual([{ type: 'markdown', value: input }])
  })

  it('takes only the first word of the info string as the language', () => {
    const [segment] = splitFences('```ts title="a.ts"\nconst a = 1\n```')
    expect(segment).toEqual({ type: 'code', lang: 'ts', code: 'const a = 1' })
  })

  it('returns no segments for empty input', () => {
    expect(splitFences('')).toEqual([])
  })

  it('survives a very long line inside a fence', () => {
    const long = 'x'.repeat(50_000)
    const [segment] = splitFences(`\`\`\`text\n${long}\n\`\`\``)
    expect(segment).toEqual({ type: 'code', lang: 'text', code: long })
  })
})
