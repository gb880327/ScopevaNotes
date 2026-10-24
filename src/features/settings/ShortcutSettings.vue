<script setup lang="ts">
import { ref } from 'vue'
import { DEFAULT_SHORTCUT, useSettingsStore } from '@/stores/settings'
import { formatAccelerator, toAccelerator } from '@/services/shortcut'

const settings = useSettingsStore()
const recorder = ref<HTMLButtonElement | null>(null)
const recording = ref(false)
const busy = ref(false)
const message = ref<string | null>(null)

function startRecording() {
  message.value = null
  recording.value = true
  recorder.value?.focus()
}

async function capture(event: KeyboardEvent) {
  if (!recording.value) return
  event.preventDefault()
  const accelerator = toAccelerator(event)
  if (!accelerator) {
    message.value = '需要 ⌘ / ⌃ / ⌥ 搭配一个字母、数字或功能键'
    return
  }
  recording.value = false
  await apply(accelerator)
}

async function apply(accelerator: string) {
  busy.value = true
  message.value = null
  try {
    await settings.changeShortcut(accelerator)
    message.value = `已生效：${formatAccelerator(accelerator)}`
  } catch {
    message.value = settings.shortcutError ?? '快捷键注册失败'
  } finally {
    busy.value = false
  }
}

async function reset() {
  await apply(DEFAULT_SHORTCUT)
}
</script>

<template>
  <section class="shortcut">
    <h3>全局快捷键</h3>

    <div class="row">
      <span class="label">显示 / 隐藏</span>
      <button
        ref="recorder"
        type="button"
        class="recorder"
        :class="{ recording }"
        :disabled="busy"
        @click="startRecording"
        @keydown="capture"
        @blur="recording = false"
      >
        {{ recording ? '请按下组合键…' : formatAccelerator(settings.shortcut) }}
      </button>
      <button
        type="button"
        class="link"
        :disabled="busy || settings.shortcut === DEFAULT_SHORTCUT"
        @click="reset"
      >
        恢复默认
      </button>
    </div>

    <p v-if="message" class="message">{{ message }}</p>
    <p class="hint">
      默认 ⇧⌘D。点击后按下新组合键立即生效；若被其他应用占用，原快捷键保持不变。
    </p>
  </section>
</template>

<style scoped>
.shortcut h3 {
  margin: 0 0 12px;
  font-size: 12px;
  font-weight: 600;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.label {
  width: 76px;
  font-size: 12px;
  color: var(--text-muted);
}

.recorder {
  min-width: 120px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-input, transparent);
  color: var(--text);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  cursor: pointer;
}

.recorder.recording {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
  color: var(--text-muted);
}

.link {
  border: 0;
  background: none;
  padding: 0;
  font-size: 11.5px;
  color: var(--accent-text);
  cursor: pointer;
}

.link:disabled {
  color: var(--text-faint);
  cursor: default;
}

.message {
  margin: 0 0 6px;
  font-size: 11.5px;
  color: var(--text-muted);
}

.hint {
  margin: 0;
  font-size: 11px;
  color: var(--text-faint);
  line-height: 1.5;
}
</style>
