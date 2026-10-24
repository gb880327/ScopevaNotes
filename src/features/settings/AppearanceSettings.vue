<script setup lang="ts">
import { useSettingsStore, type Accent, type ThemeMode } from '@/stores/settings'

const settings = useSettingsStore()

const themes: { value: ThemeMode; label: string }[] = [
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' },
  { value: 'system', label: '跟随系统' },
]

const accents: Accent[] = ['blue', 'purple', 'green', 'orange', 'graphite']
const accentColor: Record<Accent, string> = {
  blue: '#0a84ff',
  purple: '#bf5af2',
  green: '#30d158',
  orange: '#ff9f0a',
  graphite: '#8e8e93',
}
</script>

<template>
  <section class="appearance">
    <h3>外观</h3>

    <div class="row">
      <span class="label">主题</span>
      <div class="segmented">
        <button
          v-for="option in themes"
          :key="option.value"
          type="button"
          :class="{ active: settings.theme === option.value }"
          @click="settings.theme = option.value"
        >
          {{ option.label }}
        </button>
      </div>
    </div>

    <div class="row">
      <span class="label">强调色</span>
      <div class="swatches">
        <button
          v-for="accent in accents"
          :key="accent"
          type="button"
          class="swatch"
          :class="{ active: settings.accent === accent }"
          :style="{ background: accentColor[accent] }"
          :aria-label="accent"
          @click="settings.accent = accent"
        />
      </div>
    </div>

    <div class="row">
      <span class="label">编辑器字号</span>
      <input
        v-model.number="settings.editorFontSize"
        type="range"
        min="11"
        max="20"
        step="1"
      />
      <span class="value">{{ settings.editorFontSize }}px</span>
    </div>
  </section>
</template>

<style scoped>
.appearance h3 {
  margin: 0 0 12px;
  font-size: 12px;
  font-weight: 600;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.label {
  width: 76px;
  font-size: 12px;
  color: var(--text-muted);
}

.segmented {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}

.segmented button {
  border: 0;
  background: transparent;
  padding: 4px 10px;
  font-size: 11.5px;
  cursor: pointer;
  color: var(--text-muted);
}

.segmented button.active {
  background: var(--accent-soft);
  color: var(--text);
}

.swatches {
  display: flex;
  gap: 8px;
}

.swatch {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
}

.swatch.active {
  border-color: var(--text);
  outline: 2px solid var(--bg-elevated);
  outline-offset: -4px;
}

input[type='range'] {
  width: 140px;
  accent-color: var(--accent);
}

.value {
  font-size: 11px;
  color: var(--text-faint);
}
</style>
