import { watch } from 'vue'
import { useSettingsStore } from '@/stores/settings'

const DARK_QUERY = '(prefers-color-scheme: dark)'

/** Applies theme + accent + editor font size to the document root. */
export function useTheme() {
  const settings = useSettingsStore()
  const media = window.matchMedia(DARK_QUERY)

  const apply = () => {
    const resolved =
      settings.theme === 'system' ? (media.matches ? 'dark' : 'light') : settings.theme
    document.documentElement.dataset.theme = resolved
    document.documentElement.dataset.accent = settings.accent
    document.documentElement.style.setProperty(
      '--editor-font-size',
      `${settings.editorFontSize}px`,
    )
  }

  watch(
    () => [settings.theme, settings.accent, settings.editorFontSize],
    apply,
    { immediate: true },
  )

  media.addEventListener('change', apply)
  return () => media.removeEventListener('change', apply)
}
