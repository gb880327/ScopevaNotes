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

  it('treats an unterminated fence as prose', () => {
    const input = '```js\nconst a = 1'
    expect(splitFences(input)).toEqual([{ type: 'markdown', value: input }])
  })

  it('supports tilde fences', () => {
    const segments = splitFences('~~~sql\nselect 1;\n~~~')
    expect(segments).toEqual([{ type: 'code', lang: 'sql', code: 'select 1;' }])
  })
})
