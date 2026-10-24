<script setup lang="ts">
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const MIN = 220
const MAX = 420
const MIN_CONTENT = 420

let startX = 0
let startWidth = 0

function onPointerDown(event: PointerEvent) {
  event.preventDefault()
  startX = event.clientX
  startWidth = settings.sidebarWidth
  ;(event.target as HTMLElement).setPointerCapture(event.pointerId)
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp, { once: true })
}

function onPointerMove(event: PointerEvent) {
  const max = Math.min(MAX, window.innerWidth - MIN_CONTENT)
  settings.sidebarWidth = Math.min(Math.max(startWidth + event.clientX - startX, MIN), max)
}

function onPointerUp() {
  window.removeEventListener('pointermove', onPointerMove)
}
</script>

<template>
  <div class="splitter" role="separator" aria-orientation="vertical" @pointerdown="onPointerDown" />
</template>

<style scoped>
.splitter {
  width: 5px;
  margin-left: -3px;
  margin-right: -2px;
  cursor: col-resize;
  flex-shrink: 0;
  z-index: 2;
}

.splitter:hover {
  background: var(--accent-soft);
}
</style>
