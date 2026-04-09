<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useEnvironments } from '../composables/useEnvironments'
import EnvironmentForm from '../components/EnvironmentForm.vue'
import EnvironmentList from '../components/EnvironmentList.vue'
import type { CreateEnvironment } from '../api/environments'

const { environments, pending, error, load, create, remove } = useEnvironments()
const submitting = ref(false)
const submitError = ref<string | null>(null)

onMounted(async () => {
  await load()
})

async function handleCreate(data: CreateEnvironment) {
  submitting.value = true
  submitError.value = null
  try {
    await create(data)
  } catch (e: any) {
    submitError.value = e?.message ?? 'An unexpected error occurred'
  } finally {
    submitting.value = false
  }
}

async function handleDelete(name: string) {
  await remove(name)
}
</script>

<template>
  <main class="page">
    <header class="site-header">
      <h1 class="site-name">Mahakam</h1>
    </header>

    <div v-if="error" class="error-banner">
      {{ error.message }}
    </div>

    <section class="card">
      <EnvironmentForm :on-submit="handleCreate" :submitting="submitting" />
      <div v-if="submitError" class="submit-error">
        <span class="submit-error-label">Error</span>
        {{ submitError }}
      </div>
    </section>

    <div class="row-divider" />

    <section class="card">
      <div v-if="pending" class="loading">Loading...</div>
      <EnvironmentList v-else :environments="environments" :on-delete="handleDelete" />
    </section>
  </main>
</template>

<style scoped>
.page {
  max-width: 960px;
  margin: 0 auto;
  padding: 0 1.5rem 4rem;
}

.site-header {
  text-align: center;
  padding: 2.5rem 0 2rem;
  margin-bottom: 2rem;
  border-bottom: 1px solid var(--border-dim);
}

.site-name {
  font-family: 'Cinzel', 'Palatino Linotype', Palatino, serif;
  font-size: 2.6rem;
  font-weight: 700;
  color: var(--heading);
  letter-spacing: 0.18em;
}

.error-banner {
  background: var(--danger-bg);
  border: 1px solid var(--danger);
  border-left: 3px solid var(--danger);
  color: #dd8888;
  padding: 0.7rem 1rem;
  margin-bottom: 1.5rem;
  font-size: 0.88rem;
}

.card {
  background: var(--surface);
  border: 1px solid var(--border-dim);
  padding: 1.75rem 2rem;
  margin-bottom: 0.5rem;
  box-shadow: 0 2px 16px rgba(0, 0, 0, 0.35);
}

.row-divider {
  height: 1px;
  background: linear-gradient(to right, transparent, var(--border-dim), transparent);
  margin: 1.25rem 0;
}

.loading {
  text-align: center;
  color: var(--text-dim);
  padding: 2rem 0;
  font-size: 0.9rem;
}

.submit-error {
  margin-top: 1rem;
  padding: 0.65rem 1rem;
  background: var(--danger-bg);
  border: 1px solid var(--danger);
  border-left: 3px solid var(--danger);
  color: #dd8888;
  font-size: 0.85rem;
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
}

.submit-error-label {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--danger);
  flex-shrink: 0;
}
</style>
