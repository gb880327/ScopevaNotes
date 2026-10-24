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

// Fenced code block: opening fence + info string, body, closing fence of the
// same character and at least the same length.
const FENCE = /^[ \t]{0,3}(`{3,}|~{3,})([^\n]*)\n([\s\S]*?)^[ \t]{0,3}\1[ \t]*$/gm

/**
 * Splits Markdown into prose and fenced code so each code block can render its
 * own copy button (plan §1.6). Unterminated fences stay as prose.
 */
export function splitFences(markdown: string): Segment[] {
  const segments: Segment[] = []
  let cursor = 0

  for (let match = FENCE.exec(markdown); match; match = FENCE.exec(markdown)) {
    if (match.index > cursor) {
      segments.push({ type: 'markdown', value: markdown.slice(cursor, match.index) })
    }
    const info = match[2].trim()
    segments.push({
      type: 'code',
      lang: info.split(/\s+/)[0] ?? '',
      code: match[3].replace(/\n$/, ''),
    })
    cursor = match.index + match[0].length
  }

  if (cursor < markdown.length) {
    segments.push({ type: 'markdown', value: markdown.slice(cursor) })
  }
  return segments
}
