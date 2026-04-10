<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { fetchProjects, type Project } from '../api/projects'

const projects = ref<Project[]>([])
const pending = ref(false)
const error = ref<Error | null>(null)

onMounted(async () => {
  pending.value = true
  try {
    projects.value = await fetchProjects()
  } catch (e) {
    error.value = e as Error
  } finally {
    pending.value = false
  }
})
</script>

<template>
  <main class="page">
    <header class="site-header">
      <h1 class="site-name">Mahakam</h1>
      <p class="site-sub">Select a project to manage its workspaces</p>
    </header>

    <div v-if="error" class="error-banner">
      {{ error.message }}
    </div>

    <div v-if="pending" class="loading">Loading projects...</div>

    <div v-else class="project-grid">
      <NuxtLink
        v-for="project in projects"
        :key="project.name"
        :to="`/projects/${project.name}`"
        class="project-card"
      >
        <span class="project-name">{{ project.name }}</span>
        <span class="project-count">{{ project.workspace_count }} workspace{{ project.workspace_count === 1 ? '' : 's' }}</span>
      </NuxtLink>
    </div>
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

.site-sub {
  color: var(--text-dim);
  font-size: 0.88rem;
  margin-top: 0.5rem;
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

.loading {
  text-align: center;
  color: var(--text-dim);
  padding: 3rem 0;
  font-size: 0.9rem;
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 1rem;
}

.project-card {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 1.5rem 1.75rem;
  background: var(--surface);
  border: 1px solid var(--border-dim);
  text-decoration: none;
  transition: all 0.15s;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.28);
}

.project-card:hover {
  border-color: var(--accent);
  background: rgba(62, 44, 30, 0.6);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
}

.project-name {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--heading);
  letter-spacing: 0.04em;
}

.project-count {
  font-size: 0.78rem;
  color: var(--text-dim);
}
</style>
