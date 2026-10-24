import { createHighlighterCore, type HighlighterCore, type ThemedToken } from 'shiki/core'
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript'
import githubDark from 'shiki/dist/themes/github-dark.mjs'
import githubLight from 'shiki/dist/themes/github-light-high-contrast.mjs'

/** Just the token fields the preview needs. */
export interface Token {
  content: string
  color?: string
  fontStyle?: number
}

export type HighlightTheme = 'light' | 'dark'

/** Above this a block stays plain: one element per token is not free. */
export const MAX_HIGHLIGHT_CHARS = 20_000

type LanguageRegistration = Parameters<HighlighterCore['loadLanguage']>[0]

/**
 * Grammars are loaded on demand (spec: 语法高亮) so the initial bundle only
 * carries the highlighter core. Keys are canonical Shiki language ids.
 */
const LANG_LOADERS: Record<string, () => Promise<{ default: unknown }>> = {
  bash: () => import('shiki/dist/langs/bash.mjs'),
  shellscript: () => import('shiki/dist/langs/shellscript.mjs'),
  zsh: () => import('shiki/dist/langs/zsh.mjs'),
  fish: () => import('shiki/dist/langs/fish.mjs'),
  nginx: () => import('shiki/dist/langs/nginx.mjs'),
  dockerfile: () => import('shiki/dist/langs/dockerfile.mjs'),
  makefile: () => import('shiki/dist/langs/make.mjs'),
  typescript: () => import('shiki/dist/langs/typescript.mjs'),
  javascript: () => import('shiki/dist/langs/javascript.mjs'),
  tsx: () => import('shiki/dist/langs/tsx.mjs'),
  jsx: () => import('shiki/dist/langs/jsx.mjs'),
  vue: () => import('shiki/dist/langs/vue.mjs'),
  json: () => import('shiki/dist/langs/json.mjs'),
  jsonc: () => import('shiki/dist/langs/jsonc.mjs'),
  yaml: () => import('shiki/dist/langs/yaml.mjs'),
  toml: () => import('shiki/dist/langs/toml.mjs'),
  ini: () => import('shiki/dist/langs/ini.mjs'),
  sql: () => import('shiki/dist/langs/sql.mjs'),
  graphql: () => import('shiki/dist/langs/graphql.mjs'),
  rust: () => import('shiki/dist/langs/rust.mjs'),
  python: () => import('shiki/dist/langs/python.mjs'),
  go: () => import('shiki/dist/langs/go.mjs'),
  java: () => import('shiki/dist/langs/java.mjs'),
  kotlin: () => import('shiki/dist/langs/kotlin.mjs'),
  swift: () => import('shiki/dist/langs/swift.mjs'),
  ruby: () => import('shiki/dist/langs/ruby.mjs'),
  php: () => import('shiki/dist/langs/php.mjs'),
  c: () => import('shiki/dist/langs/c.mjs'),
  cpp: () => import('shiki/dist/langs/cpp.mjs'),
  csharp: () => import('shiki/dist/langs/csharp.mjs'),
  html: () => import('shiki/dist/langs/html.mjs'),
  css: () => import('shiki/dist/langs/css.mjs'),
  scss: () => import('shiki/dist/langs/scss.mjs'),
  markdown: () => import('shiki/dist/langs/markdown.mjs'),
  diff: () => import('shiki/dist/langs/diff.mjs'),
  lua: () => import('shiki/dist/langs/lua.mjs'),
  r: () => import('shiki/dist/langs/r.mjs'),
  dart: () => import('shiki/dist/langs/dart.mjs'),
  scala: () => import('shiki/dist/langs/scala.mjs'),
  elixir: () => import('shiki/dist/langs/elixir.mjs'),
  zig: () => import('shiki/dist/langs/zig.mjs'),
  regex: () => import('shiki/dist/langs/regex.mjs'),
}

/** Everyday spellings people actually type in fences. */
const ALIASES: Record<string, string> = {
  sh: 'shellscript',
  shell: 'shellscript',
  console: 'shellscript',
  terminal: 'shellscript',
  ts: 'typescript',
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  py: 'python',
  rb: 'ruby',
  rs: 'rust',
  kt: 'kotlin',
  cs: 'csharp',
  'c++': 'cpp',
  golang: 'go',
  yml: 'yaml',
  md: 'markdown',
  docker: 'dockerfile',
  make: 'makefile',
  htm: 'html',
  xhtml: 'html',
}

// The plain github-light palette drops below 4.5:1 on the code background
// (orange literals measured 3.2:1), so light uses the high-contrast variant.
const THEME_IDS: Record<HighlightTheme, 'github-light-high-contrast' | 'github-dark'> = {
  light: 'github-light-high-contrast',
  dark: 'github-dark',
}

const CACHE_LIMIT = 200

let highlighter: Promise<HighlighterCore | null> | null = null
const loadedLanguages = new Set<string>()
const cache = new Map<string, Token[][]>()

/**
 * Canonical language id for a fence tag, or `null` when unsupported — the
 * caller then renders the block as plain text (spec: 高亮失败降级).
 */
export function resolveLanguage(tag: string): string | null {
  const key = tag.trim().toLowerCase()
  if (!key) return null
  if (key in LANG_LOADERS) return key
  return ALIASES[key] ?? null
}

function getHighlighter(): Promise<HighlighterCore | null> {
  highlighter ??= createHighlighterCore({
    engine: createJavaScriptRegexEngine({ forgiving: true }),
    themes: [githubLight, githubDark],
    langs: [],
  }).catch((err: unknown) => {
    console.warn('[highlight] highlighter init failed, preview stays plain', err)
    return null
  })
  return highlighter
}

/**
 * Highlights `code` for the given theme. Never throws: any failure (unknown
 * language, broken grammar, oversized block) returns `null` so the preview
 * degrades to unhighlighted text instead of breaking.
 */
export async function highlightToTokens(
  code: string,
  lang: string,
  theme: HighlightTheme,
): Promise<Token[][] | null> {
  const canonical = resolveLanguage(lang)
  if (!canonical || !code.trim() || code.length > MAX_HIGHLIGHT_CHARS) return null

  const key = `${theme}\u0000${canonical}\u0000${code}`
  const cached = cache.get(key)
  if (cached) return cached

  try {
    const instance = await getHighlighter()
    if (!instance) return null
    if (!loadedLanguages.has(canonical)) {
      const module = await LANG_LOADERS[canonical]()
      await instance.loadLanguage(module.default as LanguageRegistration)
      loadedLanguages.add(canonical)
    }
    const tokens = instance
      .codeToTokens(code, { lang: canonical, theme: THEME_IDS[theme] })
      .tokens.map((line) =>
        line.map((token: ThemedToken) => ({
          content: token.content,
          color: token.color,
          fontStyle: token.fontStyle,
        })),
      )

    if (cache.size >= CACHE_LIMIT) {
      const oldest = cache.keys().next().value
      if (oldest !== undefined) cache.delete(oldest)
    }
    cache.set(key, tokens)
    return tokens
  } catch (err) {
    console.warn(`[highlight] "${canonical}" failed, rendering plain text`, err)
    loadedLanguages.delete(canonical)
    return null
  }
}

const ITALIC = 1
const BOLD = 2
const UNDERLINE = 4

/** Inline style for one token, driven by Shiki's highlight result. */
export function tokenStyle(token: Token): Record<string, string> {
  const style: Record<string, string> = {}
  if (token.color) style.color = token.color
  const decorations = token.fontStyle ?? 0
  if (decorations & ITALIC) style.fontStyle = 'italic'
  if (decorations & BOLD) style.fontWeight = '600'
  if (decorations & UNDERLINE) style.textDecoration = 'underline'
  return style
}
