import MarkdownIt from 'markdown-it'

/** Only these schemes may end up in an `href`; everything else renders as text. */
const SAFE_SCHEMES = new Set(['http:', 'https:', 'mailto:'])
const SCHEME = /^([a-z][a-z0-9+.-]*):/i
/** Control characters smuggle a scheme past naive checks (`java\tscript:`). */
const CONTROL_CHARS = /[\u0000-\u0020\u007f]/g

/**
 * Allowlist check for link targets (spec: 渲染安全性). Relative paths, anchors
 * and bare hosts pass; protocol-relative `//host` is rejected so the scheme can
 * never be inherited from the page.
 */
export function isSafeUrl(url: string): boolean {
  const normalized = url.replace(CONTROL_CHARS, '')
  if (!normalized) return false
  if (normalized.startsWith('//')) return false
  const scheme = SCHEME.exec(normalized)
  if (!scheme) return true
  return SAFE_SCHEMES.has(`${scheme[1].toLowerCase()}:`)
}

/** Raw HTML is disabled: untrusted Markdown must never inject markup. */
export const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
  typographer: false,
})

markdown.validateLink = isSafeUrl
