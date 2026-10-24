<script setup lang="ts">
import { ref } from 'vue'
import { copyText } from '@/services/clipboard'

const props = defineProps<{ code: string; lang: string }>()
const copied = ref(false)

async function copy() {
  try {
    await copyText(props.code)
    copied.value = true
    setTimeout(() => (copied.value = false), 1200)
  } catch (err) {
    console.error('[copy] failed', err)
  }
}
</script>

<template>
  <figure class="code-block">
    <figcaption>
      <span class="lang">{{ props.lang || 'text' }}</span>
      <button type="button" class="copy" @click="copy">
        {{ copied ? '已复制' : '复制代码' }}
      </button>
    </figcaption>
    <pre><code>{{ props.code }}</code></pre>
  </figure>
</template>

<style scoped>
.code-block {
  margin: 1em 0;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--code-bg);
  overflow: hidden;
}

figcaption {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
  color: var(--text-muted);
  text-transform: lowercase;
}

.copy {
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  border-radius: 5px;
  padding: 2px 8px;
  cursor: pointer;
  font-size: 11px;
}

.copy:hover {
  border-color: var(--border-strong);
}

pre {
  margin: 0;
  padding: 12px 14px;
  overflow: auto;
}

code {
  font-family: 'SF Mono', ui-monospace, Menlo, monospace;
  font-size: 12.5px;
  line-height: 1.6;
  white-space: pre;
}
</style>
