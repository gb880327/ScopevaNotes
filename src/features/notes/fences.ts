export interface MarkdownSegment {
  type: 'markdown'
  value: string
}

export interface CodeSegment {
  type: 'code'
  lang: string
  code: string
}

export type Segment = MarkdownSegment | CodeSegment

// Up to three spaces of indent, a run of at least three backticks or tildes,
// then an optional info string.
const OPENING_FENCE = /^ {0,3}(`{3,}|~{3,})(.*)$/
// A closing fence is the same character, at least as long, and nothing else.
const CLOSING_FENCE = /^ {0,3}(`{3,}|~{3,})[ \t]*$/

/**
 * Splits Markdown into prose and fenced code so each code block can render its
 * own copy button (plan §1.6). Follows the CommonMark fence rules: a backtick
 * info string may not contain backticks, a closing fence must use the same
 * character and be at least as long, and an unterminated fence runs to the end
 * of the document (still copyable rather than losing the block).
 */
export function splitFences(markdown: string): Segment[] {
  const lines = markdown.split('\n')
  const segments: Segment[] = []
  let prose: string[] = []

  const flushProse = () => {
    if (prose.length === 0) return
    const value = prose.join('\n')
    prose = []
    if (value) segments.push({ type: 'markdown', value })
  }

  let index = 0
  while (index < lines.length) {
    const opening = OPENING_FENCE.exec(lines[index])
    const isFence = opening !== null && !(opening[1][0] === '`' && opening[2].includes('`'))

    if (!isFence) {
      prose.push(lines[index])
      index += 1
      continue
    }

    const marker = opening[1]
    const markerChar = marker[0]
    flushProse()

    const body: string[] = []
    let cursor = index + 1
    while (cursor < lines.length) {
      const closing = CLOSING_FENCE.exec(lines[cursor])
      if (closing && closing[1][0] === markerChar && closing[1].length >= marker.length) {
        cursor += 1
        break
      }
      body.push(lines[cursor])
      cursor += 1
    }

    segments.push({ type: 'code', lang: infoLanguage(opening[2]), code: body.join('\n') })
    index = cursor
  }

  flushProse()
  return segments
}

/** First word of the info string, which is the language tag. */
function infoLanguage(info: string): string {
  return info.trim().split(/\s+/)[0] ?? ''
}
