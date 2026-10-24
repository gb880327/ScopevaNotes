import { watch } from 'vue'
import { useSettingsStore } from '@/stores/settings'

const DARK_QUERY = '(prefers-color-scheme: dark)'

/** Applies theme + accent + editor font size to the document root. */
export function useTheme() {
  const settings = useSettingsStore()
  const media =
    typeof window.matchMedia === 'function' ? window.matchMedia(DARK_QUERY) : null
  settings.systemDark = media?.matches ?? false

  const apply = () => {
    document.documentElement.dataset.theme = settings.resolvedTheme
    document.documentElement.dataset.accent = settings.accent
    document.documentElement.style.setProperty(
      '--editor-font-size',
      `${settings.editorFontSize}px`,
    )
  }

  watch(
    () => [settings.resolvedTheme, settings.accent, settings.editorFontSize],
    apply,
    { immediate: true },
  )

  const onSystemChange = (event: MediaQueryListEvent) => {
    settings.systemDark = event.matches
  }
  media?.addEventListener('change', onSystemChange)
  return () => media?.removeEventListener('change', onSystemChange)
}
