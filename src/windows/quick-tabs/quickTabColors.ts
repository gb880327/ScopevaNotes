/**
 * 速记 dot colours. Keys — not hex values — are what the database stores, so
 * the palette can be retuned without a migration (design: 圆点颜色走固定键映射).
 * The seven keys are required by the `quick-tabs` spec; keeping this list and
 * `quick_tab_repo::COLORS` in sync is what makes `setColor` validation total.
 */
export interface QuickTabColorSpec {
  /** Dot fill. Bold by design: dots are identity, not text. */
  dot: string
  /** Chinese name, used for the accessible label. */
  name: string
}

export const QUICK_TAB_COLORS: Record<string, QuickTabColorSpec> = {
  yellow: { dot: '#ffd60a', name: '黄' },
  orange: { dot: '#ff9f0a', name: '橙' },
  red: { dot: '#ff375f', name: '红' },
  purple: { dot: '#bf5af2', name: '紫' },
  blue: { dot: '#0a84ff', name: '蓝' },
  cyan: { dot: '#64d2ff', name: '青' },
  green: { dot: '#30d158', name: '绿' },
}

export const QUICK_TAB_COLOR_KEYS = Object.keys(QUICK_TAB_COLORS)

const FALLBACK: QuickTabColorSpec = { dot: '#8e8e93', name: '灰' }

export function colorSpec(key: string): QuickTabColorSpec {
  return QUICK_TAB_COLORS[key] ?? FALLBACK
}

export function dotColor(key: string): string {
  return colorSpec(key).dot
}

/** Accessible name for a dot: colour + title, so the bar is not colour-only. */
export function dotLabel(key: string, title: string): string {
  const name = colorSpec(key).name
  const text = title.trim() || '未命名'
  return `速记 ${text}（${name}）`
}
