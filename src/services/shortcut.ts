const MODIFIER_SYMBOLS: Record<string, string> = {
  Command: '⌘',
  Cmd: '⌘',
  Super: '⌘',
  CommandOrControl: '⌘',
  Control: '⌃',
  Ctrl: '⌃',
  Alt: '⌥',
  Option: '⌥',
  Shift: '⇧',
}

const SYMBOL_ORDER = ['⌃', '⌥', '⇧', '⌘']

const NAMED_KEYS: Record<string, string> = {
  ' ': 'Space',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Escape: 'Escape',
  Delete: 'Delete',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  ',': 'Comma',
  '.': 'Period',
  '/': 'Slash',
  ';': 'Semicolon',
  "'": 'Quote',
  '[': 'BracketLeft',
  ']': 'BracketRight',
  '-': 'Minus',
  '=': 'Equal',
  '`': 'Backquote',
  '\\': 'Backslash',
}

const MODIFIER_EVENT_KEYS = new Set(['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'Fn', 'Dead'])

/** "CommandOrControl+Shift+D" → "⇧⌘D" (macOS symbol order). */
export function formatAccelerator(accelerator: string): string {
  const parts = accelerator
    .split('+')
    .map((part) => part.trim())
    .filter(Boolean)
  const symbols = parts
    .map((part) => MODIFIER_SYMBOLS[part])
    .filter((symbol): symbol is string => Boolean(symbol))
    .sort((a, b) => SYMBOL_ORDER.indexOf(a) - SYMBOL_ORDER.indexOf(b))
  const keys = parts
    .filter((part) => !MODIFIER_SYMBOLS[part])
    .map((key) => (key.length === 1 ? key.toUpperCase() : key))
  return [...symbols, ...keys].join('')
}

function keyName(event: KeyboardEvent): string | null {
  if (/^F([1-9]|1\d|20)$/.test(event.key)) return event.key.toUpperCase()
  if (NAMED_KEYS[event.key]) return NAMED_KEYS[event.key]
  if (event.key.length === 1) {
    if (/[a-z]/i.test(event.key)) return event.key.toUpperCase()
    if (/\d/.test(event.key)) return event.key
  }
  return null
}

/**
 * Converts a keydown into a Tauri accelerator string. Returns null while the
 * user is still holding only modifiers, or for combinations that make no sense
 * as a global shortcut (no ⌘/⌃/⌥, or an unmappable key).
 */
export function toAccelerator(event: KeyboardEvent): string | null {
  if (MODIFIER_EVENT_KEYS.has(event.key)) return null
  const modifiers: string[] = []
  if (event.metaKey) modifiers.push('Command')
  if (event.ctrlKey) modifiers.push('Control')
  if (event.altKey) modifiers.push('Alt')
  if (event.shiftKey) modifiers.push('Shift')
  if (!modifiers.some((mod) => mod !== 'Shift')) return null
  const key = keyName(event)
  if (!key) return null
  return [...modifiers, key].join('+')
}
