<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import WorkspaceForm from '../../components/WorkspaceForm.vue'
import WorkspaceList from '../../components/WorkspaceList.vue'
import type { CreateWorkspace } from '../../api/workspaces'

const route = useRoute()
const project = route.params.project as string

const { data: workspaces, refresh, pending, error } = await useFetch<any[]>(`/api/projects/${project}/workspaces`)

const submitting = ref(false)
const submitError = ref<string | null>(null)

async function handleCreate(data: CreateWorkspace) {
  submitting.value = true
  submitError.value = null
  try {
    await $fetch('/api/workspaces', {
      method: 'POST',
      body: { ...data, project },
    })
    await refresh()
  } catch (e: any) {
    submitError.value = e?.message ?? 'An unexpected error occurred'
  } finally {
    submitting.value = false
  }
}

async function handleDelete(name: string) {
  await $fetch(`/api/workspaces/${name}`, { method: 'DELETE' })
  await refresh()
}
</script>

<template>
  <main class="page">
    <header class="page-header">
      <NuxtLink to="/" class="back-link">← Projects</NuxtLink>
      <h1 class="page-title">{{ project }}</h1>
    </header>

    <div v-if="error" class="error-banner">
      {{ (error as any).message }}
    </div>

    <section class="card">
      <WorkspaceForm :on-submit="handleCreate" :submitting="submitting" />
      <div v-if="submitError" class="submit-error">
        <span class="submit-error-label">Error</span>
        {{ submitError }}
      </div>
    </section>

    <div class="row-divider" />

    <section class="card">
      <div v-if="pending" class="loading">Loading...</div>
      <WorkspaceList
        v-else
        :workspaces="workspaces ?? []"
        :on-delete="handleDelete"
        :project="project"
      />
    </section>
  </main>
</template>

<style scoped>
.page {
  max-width: 960px;
  margin: 0 auto;
  padding: 0 1.5rem 4rem;
}

.page-header {
  padding: 1.75rem 0 1.5rem;
  margin-bottom: 2rem;
  border-bottom: 1px solid var(--border-dim);
  display: flex;
  align-items: center;
  gap: 1.25rem;
}

.back-link {
  font-size: 0.8rem;
  color: var(--text-dim);
  text-decoration: none;
  transition: color 0.12s;
  flex-shrink: 0;
}

.back-link:hover {
  color: var(--accent);
}

.page-title {
  font-family: 'Cinzel', 'Palatino Linotype', Palatino, serif;
  font-size: 1.7rem;
  font-weight: 700;
  color: var(--heading);
  letter-spacing: 0.12em;
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
