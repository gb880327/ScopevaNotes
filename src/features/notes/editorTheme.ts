import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import type { Extension } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import { tags } from '@lezer/highlight'

export type EditorThemeName = 'light' | 'dark'

/**
 * Token colours mirror the Shiki themes the preview uses (`github-light-high-contrast`,
 * `github-dark`) but are re-tuned for the editor background (`--bg`), which is
 * `#ffffff` / `#1c1c1e` rather than Shiki's own `#ffffff` / `#24292e`.
 * Every entry is asserted to keep at least 4.5:1 against that background in
 * `editorTheme.test.ts`.
 */
export const EDITOR_PALETTES: Record<EditorThemeName, Record<PaletteKey, string>> = {
  light: {
    text: '#0e1116',
    heading: '#023b95',
    link: '#032563',
    inlineCode: '#023b95',
    quote: '#024c1a',
    listMarker: '#702c00',
    marker: '#66707b',
    keyword: '#a0111f',
    string: '#032563',
    number: '#a0111f',
  },
  dark: {
    text: '#e1e4e8',
    heading: '#79b8ff',
    link: '#dbedff',
    inlineCode: '#79b8ff',
    quote: '#85e89d',
    listMarker: '#ffab70',
    marker: '#8b949e',
    keyword: '#f97583',
    string: '#9ecbff',
    number: '#79b8ff',
  },
}

export type PaletteKey =
  | 'text'
  | 'heading'
  | 'link'
  | 'inlineCode'
  | 'quote'
  | 'listMarker'
  | 'marker'
  | 'keyword'
  | 'string'
  | 'number'

/** Backgrounds the editor renders on, mirroring `styles/tokens.css`. */
export const EDITOR_BACKGROUNDS: Record<EditorThemeName, string> = {
  light: '#ffffff',
  dark: '#1c1c1e',
}

export function buildHighlightStyle(theme: EditorThemeName): HighlightStyle {
  const p = EDITOR_PALETTES[theme]
  return HighlightStyle.define([
    { tag: tags.heading1, color: p.heading, fontWeight: '700' },
    { tag: tags.heading2, color: p.heading, fontWeight: '600' },
    { tag: tags.heading3, color: p.heading, fontWeight: '600' },
    { tag: [tags.heading4, tags.heading5, tags.heading6], color: p.heading },
    { tag: tags.strong, color: p.text, fontWeight: '700' },
    { tag: tags.emphasis, color: p.text, fontStyle: 'italic' },
    { tag: tags.strikethrough, color: p.marker, textDecoration: 'line-through' },
    { tag: tags.link, color: p.link, textDecoration: 'underline' },
    { tag: tags.url, color: p.link },
    { tag: [tags.labelName, tags.string], color: p.string },
    { tag: tags.monospace, color: p.inlineCode },
    { tag: tags.quote, color: p.quote, fontStyle: 'italic' },
    { tag: tags.list, color: p.listMarker },
    { tag: tags.contentSeparator, color: p.marker },
    {
      tag: [tags.processingInstruction, tags.punctuation, tags.meta, tags.comment],
      color: p.marker,
    },
    { tag: tags.keyword, color: p.keyword },
    { tag: tags.number, color: p.number },
    { tag: tags.bool, color: p.number },
    { tag: tags.atom, color: p.number },
    { tag: tags.operator, color: p.marker },
    { tag: tags.function(tags.variableName), color: p.inlineCode },
    { tag: tags.typeName, color: p.heading },
    { tag: tags.className, color: p.heading },
    { tag: tags.propertyName, color: p.inlineCode },
    { tag: tags.invalid, color: p.keyword, textDecoration: 'underline wavy' },
  ])
}

/**
 * Editor chrome (gutters, selection, search panel, brackets) expressed entirely
 * with design tokens, so it follows the theme and accent colour without being
 * reconfigured. Only syntax colours are swapped per theme.
 */
export const editorChrome: Extension = EditorView.theme({
  '&': {
    fontSize: 'var(--editor-font-size)',
    backgroundColor: 'transparent',
    color: 'var(--text)',
    height: '100%',
  },
  '.cm-content': {
    fontFamily: "'SF Mono', ui-monospace, Menlo, monospace",
    padding: '8px 0 40px',
    caretColor: 'var(--accent)',
  },
  '.cm-scroller': { overflow: 'auto', lineHeight: '1.65' },
  // `--text-faint` measures 3.3:1 on white, so line numbers use the muted
  // token instead and the current line gets full text colour.
  '.cm-gutters': {
    backgroundColor: 'transparent',
    border: 'none',
    color: 'var(--text-muted)',
  },
  '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--text)' },
  '&.cm-focused': { outline: 'none' },
  '.cm-activeLine': { backgroundColor: 'var(--hover)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--accent)' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'var(--accent-soft)',
  },
  '.cm-selectionMatch': { backgroundColor: 'var(--selected)' },
  '.cm-searchMatch': {
    backgroundColor: 'var(--accent-soft)',
    outline: '1px solid var(--accent)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: 'var(--selected)' },
  '.cm-panels': {
    backgroundColor: 'var(--bg-elevated)',
    color: 'var(--text)',
    borderColor: 'var(--border)',
  },
  '.cm-panels.cm-panels-bottom': { borderTop: '1px solid var(--border)' },
  '.cm-textfield': {
    backgroundColor: 'var(--bg-input)',
    color: 'var(--text)',
    border: '1px solid var(--border)',
    borderRadius: '6px',
  },
  '.cm-button': {
    backgroundImage: 'none',
    backgroundColor: 'var(--bg-input)',
    color: 'var(--text)',
    border: '1px solid var(--border)',
    borderRadius: '6px',
  },
  '.cm-tooltip': {
    backgroundColor: 'var(--bg-elevated)',
    border: '1px solid var(--border)',
    color: 'var(--text)',
  },
  '.cm-foldPlaceholder': {
    backgroundColor: 'var(--bg-input)',
    border: 'none',
    color: 'var(--text-muted)',
  },
  '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': {
    backgroundColor: 'var(--accent-soft)',
    outline: 'none',
  },
  '.cm-nonmatchingBracket': { color: 'var(--danger)' },
})

/** Theme-dependent part of the editor, swapped through a `Compartment`. */
export function editorSyntaxTheme(theme: EditorThemeName): Extension {
  return syntaxHighlighting(buildHighlightStyle(theme))
}
