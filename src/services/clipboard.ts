import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { isDesktop } from './commands'

/** Clipboard access for code blocks and "copy full markdown". */
export async function copyText(value: string): Promise<void> {
  if (isDesktop) {
    await writeText(value)
    return
  }
  await navigator.clipboard.writeText(value)
}
