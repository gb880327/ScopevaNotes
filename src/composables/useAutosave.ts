/**
 * Debounced, serialised save queue.
 *
 * Contract (plan §5): switching notes, hiding the window, switching libraries
 * and quitting must `flush()` before proceeding, so a pending edit can never
 * land in the wrong note.
 */
export interface AutosaveQueue {
  schedule: () => void
  flush: () => Promise<void>
  isDirty: () => boolean
}

export function createAutosaveQueue(run: () => Promise<void>, delay = 600): AutosaveQueue {
  let timer: ReturnType<typeof setTimeout> | null = null
  let dirty = false
  let chain: Promise<void> = Promise.resolve()

  const clearTimer = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  const flush = (): Promise<void> => {
    clearTimer()
    if (!dirty) return chain
    dirty = false
    chain = chain
      .then(() => run())
      .catch((err) => {
        // Keep the dirty flag so the next flush retries instead of losing data.
        dirty = true
        console.error('[autosave] save failed', err)
      })
    return chain
  }

  const schedule = () => {
    dirty = true
    clearTimer()
    timer = setTimeout(() => void flush(), delay)
  }

  return { schedule, flush, isDirty: () => dirty }
}
