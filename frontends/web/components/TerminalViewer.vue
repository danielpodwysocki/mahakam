<script setup lang="ts">
defineProps<{ envName: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <div class="console-backdrop" @click.self="emit('close')">
    <div class="console-panel">
      <header class="console-header">
        <div class="console-title">
          <span class="console-icon">▶_</span>
          <span>{{ envName }}</span>
        </div>
        <button class="console-close" @click="emit('close')">✕ Dismiss</button>
      </header>
      <iframe
        class="console-frame"
        :src="`/projects/viewers/${envName}/`"
        :title="`console session: ${envName}`"
        allowfullscreen
      />
    </div>
  </div>
</template>

<style scoped>
.console-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  background: rgba(5, 3, 2, 0.78);
  display: flex;
  align-items: center;
  justify-content: center;
}

.console-panel {
  width: 92%;
  height: 86vh;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border: 1px solid var(--border-dim);
  box-shadow: 0 8px 48px rgba(0, 0, 0, 0.7), 0 0 0 1px rgba(200, 121, 65, 0.15);
}

.console-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.55rem 1rem;
  background: var(--input-bg);
  border-bottom: 1px solid var(--border-dim);
  flex-shrink: 0;
}

.console-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.82rem;
  color: var(--heading);
  font-weight: 600;
  letter-spacing: 0.06em;
}

.console-icon {
  color: var(--ember);
  font-family: 'Courier New', Courier, monospace;
  font-size: 0.78rem;
}

.console-close {
  font-family: inherit;
  font-size: 0.72rem;
  padding: 0.25rem 0.7rem;
  background: transparent;
  border: 1px solid rgba(200, 68, 68, 0.4);
  color: var(--danger);
  cursor: pointer;
  transition: all 0.15s;
  letter-spacing: 0.04em;
}

.console-close:hover {
  background: var(--danger-bg);
  border-color: var(--danger);
}

.console-frame {
  flex: 1;
  width: 100%;
  border: none;
  background: #0d0806;
}
</style>
