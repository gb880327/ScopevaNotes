import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

/**
 * Accessibility audit for the design tokens (task 7.4).
 *
 * Every text token is checked against every surface it can be drawn on, for both
 * themes and all five accent colours. Surfaces that are translucent in
 * `tokens.css` are composited over the page background first, which is what the
 * browser does.
 */

const css = readFileSync(resolve(process.cwd(), 'src/styles/tokens.css'), 'utf8')

const THEMES = ['light', 'dark'] as const
const ACCENTS = ['blue', 'purple', 'green', 'orange', 'graphite'] as const
type Theme = (typeof THEMES)[number]
type Accent = (typeof ACCENTS)[number]

type Vars = Record<string, string>

function block(selector: string): Vars {
  const start = css.indexOf(`${selector} {`)
  if (start < 0) throw new Error(`missing block: ${selector}`)
  const end = css.indexOf('}', start)
  const vars: Vars = {}
  for (const match of css.slice(start, end).matchAll(/--([\w-]+):\s*([^;]+);/g)) {
    vars[match[1]] = match[2].trim()
  }
  return vars
}

const root: Vars = block(':root')
const darkVars: Vars = block("[data-theme='dark']")
const accentVars: Record<Accent, Vars> = {
  blue: {},
  purple: block("[data-accent='purple']"),
  green: block("[data-accent='green']"),
  orange: block("[data-accent='orange']"),
  graphite: block("[data-accent='graphite']"),
}
const darkAccentVars: Record<Accent, Vars> = {
  blue: {},
  purple: block("[data-theme='dark'][data-accent='purple']"),
  green: block("[data-theme='dark'][data-accent='green']"),
  orange: block("[data-theme='dark'][data-accent='orange']"),
  graphite: block("[data-theme='dark'][data-accent='graphite']"),
}

export function resolveTokens(theme: Theme, accent: Accent): Vars {
  return {
    ...root,
    ...(theme === 'dark' ? darkVars : {}),
    ...accentVars[accent],
    ...(theme === 'dark' ? darkAccentVars[accent] : {}),
  }
}

function parseColor(value: string): { rgb: [number, number, number]; alpha: number } {
  const hex = value.trim().match(/^#([0-9a-f]{6})$/i)
  if (hex) {
    const [r, g, b] = [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16)) as [
      number,
      number,
      number,
    ]
    return { rgb: [r, g, b], alpha: 1 }
  }
  const rgba = value.trim().match(/^rgba?\(([^)]+)\)$/)
  if (rgba) {
    const parts = rgba[1].split(',').map((part) => Number(part.trim()))
    return { rgb: [parts[0], parts[1], parts[2]], alpha: parts[3] ?? 1 }
  }
  throw new Error(`unsupported colour: ${value}`)
}

function composite(over: string, base: [number, number, number]): [number, number, number] {
  const { rgb, alpha } = parseColor(over)
  return rgb.map((value, index) => Math.round(value * alpha + base[index] * (1 - alpha))) as [
    number,
    number,
    number,
  ]
}

function channel(value: number): number {
  const c = value / 255
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

function luminance(rgb: [number, number, number]): number {
  return 0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
}

function ratio(foreground: string, background: [number, number, number]): number {
  const [light, dark] = [luminance(parseColor(foreground).rgb), luminance(background)].sort(
    (a, b) => b - a,
  )
  return (light + 0.05) / (dark + 0.05)
}

/** Surfaces text can sit on. Translucent ones are composited over --bg. */
function surfaces(vars: Vars): Record<string, [number, number, number]> {
  const page = parseColor(vars.bg).rgb
  return {
    bg: page,
    'bg-sidebar': parseColor(vars['bg-sidebar']).rgb,
    'bg-elevated': parseColor(vars['bg-elevated']).rgb,
    'bg-input': composite(vars['bg-input'], page),
    selected: composite(vars.selected, page),
    'accent-soft': composite(vars['accent-soft'], page),
    'code-bg': parseColor(vars['code-bg']).rgb,
  }
}

const TEXT_TOKENS = ['text', 'text-muted', 'text-faint', 'accent-text', 'danger']
const MIN_RATIO = 4.5

describe('design token contrast', () => {
  for (const theme of THEMES) {
    for (const accent of ACCENTS) {
      it(`keeps every text token readable (${theme}, ${accent})`, () => {
        const vars = resolveTokens(theme, accent)
        const backgrounds = surfaces(vars)
        const failures: string[] = []
        for (const token of TEXT_TOKENS) {
          const value = vars[token]
          expect(value, `missing --${token}`).toBeTruthy()
          for (const [name, background] of Object.entries(backgrounds)) {
            const value_ = ratio(value, background)
            if (value_ < MIN_RATIO) {
              failures.push(
                `--${token} ${value} on --${name}: ${value_.toFixed(2)}:1 (< ${MIN_RATIO})`,
              )
            }
          }
        }
        expect(failures).toEqual([])
      })
    }

    it(`keeps filled buttons readable (${theme})`, () => {
      for (const accent of ACCENTS) {
        const vars = resolveTokens(theme, accent)
        const background = parseColor(vars['accent-text']).rgb
        const value = ratio(vars['accent-contrast'], background)
        expect(
          value,
          `--accent-contrast on --accent-text (${theme}, ${accent})`,
        ).toBeGreaterThanOrEqual(MIN_RATIO)
      }
    })
  }

  it('defines the accent text colour for every accent in both themes', () => {
    for (const theme of THEMES) {
      for (const accent of ACCENTS) {
        expect(resolveTokens(theme, accent)['accent-text']).toBeTruthy()
      }
    }
  })
})
