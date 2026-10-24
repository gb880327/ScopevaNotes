import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/services/commands', () => ({
  isDesktop: false,
  api: {},
  errorMessage: (error: unknown) => String(error),
  CommandError: class extends Error {},
}))

import { createAutosaveQueue } from './useAutosave'

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('autosave queue', () => {
  it('collapses a burst of keystrokes into one save', async () => {
    const run = vi.fn(async () => {})
    const queue = createAutosaveQueue(run, 600)

    for (let i = 0; i < 20; i += 1) {
      queue.schedule()
      await vi.advanceTimersByTimeAsync(50)
    }
    expect(run).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(600)
    expect(run).toHaveBeenCalledTimes(1)
    expect(queue.isDirty()).toBe(false)
  })

  it('serialises saves so a second run cannot start before the first finishes', async () => {
    let release = () => {}
    const order: string[] = []
    const run = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            release = () => {
              order.push('first done')
              resolve()
            }
          }),
      )
      .mockImplementationOnce(async () => {
        order.push('second start')
      })

    const queue = createAutosaveQueue(run, 0)
    queue.schedule()
    await vi.advanceTimersByTimeAsync(0)
    queue.schedule()
    await vi.advanceTimersByTimeAsync(0)

    expect(order).toEqual([])
    release()
    await queue.flush()
    expect(order).toEqual(['first done', 'second start'])
    expect(run).toHaveBeenCalledTimes(2)
  })

  it('keeps the dirty flag after a failed save so a later flush retries', async () => {
    const run = vi.fn().mockRejectedValueOnce(new Error('disk full')).mockResolvedValue(undefined)
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const queue = createAutosaveQueue(run, 0)

    queue.schedule()
    await vi.advanceTimersByTimeAsync(0)
    expect(queue.isDirty()).toBe(true)

    await queue.flush()
    expect(run).toHaveBeenCalledTimes(2)
    expect(queue.isDirty()).toBe(false)
    spy.mockRestore()
  })

  it('drops pending work on discard', async () => {
    const run = vi.fn(async () => {})
    const queue = createAutosaveQueue(run, 600)

    queue.schedule()
    queue.discard()
    await vi.advanceTimersByTimeAsync(1000)

    expect(run).not.toHaveBeenCalled()
    expect(queue.isDirty()).toBe(false)
    await queue.flush()
    expect(run).not.toHaveBeenCalled()
  })

  it('does nothing on flush when there is no pending edit', async () => {
    const run = vi.fn(async () => {})
    const queue = createAutosaveQueue(run, 600)

    await queue.flush()
    expect(run).not.toHaveBeenCalled()
  })
})
