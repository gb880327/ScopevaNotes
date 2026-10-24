<script setup lang="ts">
import { computed, ref } from 'vue'
import { useNotesStore } from '@/stores/notes'
import TagEditor from './TagEditor.vue'

const notes = useNotesStore()
const adding = ref(false)

const total = computed(() => notes.tags.reduce((sum, tag) => sum + tag.noteCount, 0))
</script>

<template>
  <section class="tags">
    <header>
      <span class="label">标签</span>
      <button type="button" class="add" title="新建标签" @click="adding = !adding">＋</button>
    </header>

    <TagEditor v-if="adding" @done="adding = false" />

    <div class="scroll">
      <button
        type="button"
        class="tag"
        :class="{ active: !notes.activeTagIds.length }"
        @click="notes.clearTagFilters()"
      >
        <span class="name">全部标签</span>
        <span class="count">{{ total }}</span>
      </button>
      <button
        v-for="tag in notes.tags"
        :key="tag.id"
        type="button"
        class="tag"
        :class="{ active: notes.activeTagIds.includes(tag.id) }"
        @click="notes.toggleTagFilter(tag.id)"
      >
        <span class="dot" :style="{ background: tag.color ?? 'var(--accent)' }" />
        <span class="name">{{ tag.name }}</span>
        <span class="count">{{ tag.noteCount }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.tags {
  border-top: 1px solid var(--border);
  max-height: 38%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px 4px;
}

.label {
  font-size: 10.5px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-faint);
}

.add {
  border: 0;
  background: none;
  color: var(--text-faint);
  cursor: pointer;
  font-size: 13px;
}

.add:hover {
  color: var(--text);
}

.scroll {
  overflow-y: auto;
  padding: 0 8px 8px;
}

.tag {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  border: 0;
  background: transparent;
  border-radius: 6px;
  padding: 4px 6px;
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}

.tag:hover {
  background: var(--hover);
}

.tag.active {
  background: var(--selected);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.name {
  flex: 1;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.count {
  color: var(--text-faint);
  font-size: 10.5px;
}
</style>
