import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  EDITOR_BACKGROUNDS,
  EDITOR_PALETTES,
  buildHighlightStyle,
  editorChrome,
  editorSyntaxTheme,
  type EditorThemeName,
} from './editorTheme'

const tokensCss = readFileSync(resolve(process.cwd(), 'src/styles/tokens.css'), 'utf8')

function token(block: string, name: string): string | undefined {
  const match = block.match(new RegExp(`${name}:\\s*([^;]+);`))
  return match?.[1].trim()
}

function tokenBlock(selector: string): string {
  const start = tokensCss.indexOf(`${selector} {`)
  expect(start, `missing ${selector} block in tokens.css`).toBeGreaterThanOrEqual(0)
  const end = tokensCss.indexOf('}', start)
  return tokensCss.slice(start, end)
}

function channel(value: number): number {
  const c = value / 255
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

function luminance(hex: string): number {
  const [r, g, b] = hex.replace('#', '').match(/../g)!.map((pair) => channel(parseInt(pair, 16)))
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

function contrast(foreground: string, background: string): number {
  const [light, dark] = [luminance(foreground), luminance(background)].sort((a, b) => b - a)
  return (light + 0.05) / (dark + 0.05)
}

const themes: EditorThemeName[] = ['light', 'dark']

describe('editor theme palettes', () => {
  it('uses the same backgrounds as tokens.css', () => {
    expect(EDITOR_BACKGROUNDS.light).toBe(token(tokenBlock(':root'), '--bg'))
    expect(EDITOR_BACKGROUNDS.dark).toBe(token(tokenBlock("[data-theme='dark']"), '--bg'))
  })

  for (const theme of themes) {
    it(`keeps every ${theme} token at or above 4.5:1`, () => {
      const background = EDITOR_BACKGROUNDS[theme]
      for (const [name, color] of Object.entries(EDITOR_PALETTES[theme])) {
        const ratio = contrast(color, background)
        expect(ratio, `${theme}.${name} ${color} on ${background}`).toBeGreaterThanOrEqual(4.5)
      }
    })
  }

  it('differs between light and dark for every role', () => {
    for (const role of Object.keys(EDITOR_PALETTES.light) as Array<keyof typeof EDITOR_PALETTES.light>) {
      expect(EDITOR_PALETTES.dark[role]).not.toBe(EDITOR_PALETTES.light[role])
    }
  })
})

describe('editor extensions', () => {
  it('builds a highlight style without falling back to defaults', () => {
    const light = buildHighlightStyle('light')
    const dark = buildHighlightStyle('dark')
    expect(light.specs.length).toBeGreaterThan(10)
    expect(dark.specs.length).toBe(light.specs.length)
    expect(light.style).not.toBe(dark.style)
    // basicSetup registers a fallback style that is tuned for light backgrounds,
    // so every tag we render must be covered explicitly by our own style.
    expect(light.specs.every((spec) => spec.tag && spec.color)).toBe(true)
  })

  it('returns extensions for both themes', () => {
    expect(Array.isArray(editorSyntaxTheme('light'))).toBe(true)
    expect(Array.isArray(editorSyntaxTheme('dark'))).toBe(true)
    expect(Array.isArray(editorChrome)).toBe(true)
  })
})
