import { onBeforeUnmount, onMounted } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'

interface KeyboardHandlers {
  focusSearch: () => void
}

function isTypingTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null
  if (!element) return false
  return (
    element.tagName === 'INPUT' ||
    element.tagName === 'TEXTAREA' ||
    element.isContentEditable ||
    element.closest('.cm-editor') !== null
  )
}

/** Window-level shortcuts. Native menu accelerators live in Rust. */
export function useKeyboard(handlers: KeyboardHandlers) {
  const notes = useNotesStore()
  const ui = useUiStore()

  const onKeydown = (event: KeyboardEvent) => {
    const mod = event.metaKey || event.ctrlKey
    if (!mod) {
      if (event.key === 'Escape' && ui.viewMode === 'edit') {
        event.preventDefault()
        void notes.flush().then(() => (ui.viewMode = 'read'))
      }
      return
    }

    switch (event.key.toLowerCase()) {
      case 'f':
        event.preventDefault()
        handlers.focusSearch()
        break
      case 'n':
        if (isTypingTarget(event.target)) return
        event.preventDefault()
        void notes.createNote()
        break
      case 'e':
        event.preventDefault()
        if (ui.viewMode === 'edit') {
          void notes.flush().then(() => (ui.viewMode = 'read'))
        } else {
          ui.viewMode = 'edit'
        }
        break
      case ',':
        event.preventDefault()
        ui.openSettings()
        break
      default:
        break
    }
  }

  onMounted(() => window.addEventListener('keydown', onKeydown))
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
}
