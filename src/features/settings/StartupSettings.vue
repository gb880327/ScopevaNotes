<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { errorMessage } from '@/services/commands'

const enabled = ref(false)
const busy = ref(false)
const message = ref<string | null>(null)

/** Always re-read from the system: the tray switch can change it too. */
async function refresh() {
  try {
    enabled.value = await isEnabled()
  } catch (err) {
    message.value = errorMessage(err)
  }
}

async function toggle() {
  busy.value = true
  message.value = null
  try {
    if (enabled.value) await disable()
    else await enable()
  } catch (err) {
    message.value = errorMessage(err)
  } finally {
    await refresh()
    busy.value = false
  }
}

onMounted(refresh)
</script>

<template>
  <section>
    <h3>启动</h3>
    <label class="row">
      <input type="checkbox" :checked="enabled" :disabled="busy" @change="toggle" />
      <span>登录时自动启动并驻留菜单栏</span>
    </label>
    <p v-if="message" class="message">{{ message }}</p>
    <p class="hint">显示的是系统里 LaunchAgent 的实际状态，与菜单栏开关一致。</p>
  </section>
</template>

<style scoped>
h3 {
  margin: 0 0 12px;
  font-size: 12px;
  font-weight: 600;
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
}

.message {
  margin: 8px 0 0;
  font-size: 11.5px;
  color: var(--text-muted);
}

.hint {
  margin: 8px 0 0;
  font-size: 11px;
  color: var(--text-faint);
}
</style>
