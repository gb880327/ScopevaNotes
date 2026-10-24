import { describe, expect, it } from 'vitest'
import { formatAccelerator, toAccelerator } from './shortcut'

function keydown(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent('keydown', init)
}

describe('formatAccelerator', () => {
  it('renders modifier symbols in macOS order', () => {
    expect(formatAccelerator('CommandOrControl+Shift+D')).toBe('⇧⌘D')
    expect(formatAccelerator('Shift+Command+Alt+Control+K')).toBe('⌃⌥⇧⌘K')
  })

  it('keeps named keys readable', () => {
    expect(formatAccelerator('Command+Space')).toBe('⌘Space')
    expect(formatAccelerator('Command+Shift+1')).toBe('⇧⌘1')
  })
})

describe('toAccelerator', () => {
  it('captures command combinations', () => {
    expect(toAccelerator(keydown({ key: 'd', metaKey: true, shiftKey: true }))).toBe(
      'Command+Shift+D',
    )
  })

  it('ignores modifier-only presses', () => {
    expect(toAccelerator(keydown({ key: 'Shift', shiftKey: true }))).toBeNull()
    expect(toAccelerator(keydown({ key: 'Meta', metaKey: true }))).toBeNull()
  })

  it('rejects shift-only and unmodified keys', () => {
    expect(toAccelerator(keydown({ key: 'D', shiftKey: true }))).toBeNull()
    expect(toAccelerator(keydown({ key: 'D' }))).toBeNull()
  })

  it('maps special keys', () => {
    expect(toAccelerator(keydown({ key: ' ', metaKey: true }))).toBe('Command+Space')
    expect(toAccelerator(keydown({ key: 'ArrowUp', ctrlKey: true }))).toBe('Control+Up')
    expect(toAccelerator(keydown({ key: 'F5', altKey: true }))).toBe('Alt+F5')
  })
})
