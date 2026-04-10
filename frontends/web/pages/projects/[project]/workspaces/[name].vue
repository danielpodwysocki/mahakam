<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { fetchWorkspace, fetchWorkspaceMetrics, type Workspace, type WsMetrics } from '../../../../api/workspaces'

const route = useRoute()
const project = route.params.project as string
const wsName = route.params.name as string

const workspace = ref<Workspace | null>(null)
const metrics = ref<WsMetrics | null>(null)
const wsError = ref<string | null>(null)
const metricsError = ref<string | null>(null)
const activeTab = ref<'dashboard' | string>('dashboard')
const iframeRef = ref<HTMLIFrameElement | null>(null)

let pollTimer: ReturnType<typeof setInterval> | null = null

async function loadWorkspace() {
  try {
    workspace.value = await fetchWorkspace(wsName)
  } catch (e: any) {
    wsError.value = e.message
  }
}

async function loadMetrics() {
  try {
    metrics.value = await fetchWorkspaceMetrics(wsName)
  } catch {
    // metrics may not be available while workspace is pending
  }
}

onMounted(async () => {
  await Promise.all([loadWorkspace(), loadMetrics()])
  // Poll until settled
  pollTimer = setInterval(async () => {
    await loadWorkspace()
    if (workspace.value?.status === 'ready' || workspace.value?.status === 'failed') {
      clearInterval(pollTimer!)
      pollTimer = null
      if (workspace.value?.status === 'ready') {
        await loadMetrics()
      }
    }
  }, 3000)
})

onUnmounted(() => {
  if (pollTimer !== null) clearInterval(pollTimer)
})

const activeViewer = computed(() =>
  workspace.value?.viewers.find((v) => v.name === activeTab.value) ?? null,
)

function switchViewer(name: string) {
  // Suppress noVNC's beforeunload dialog before the iframe navigates away.
  // A capturing listener runs before noVNC's (non-capture) handler; we
  // neutralise preventDefault and make returnValue a no-op so the browser
  // never sees a cancellation request.
  const frame = iframeRef.value
  if (frame?.contentWindow) {
    try {
      frame.contentWindow.addEventListener(
        'beforeunload',
        (e: BeforeUnloadEvent) => {
          e.preventDefault = () => {}
          Object.defineProperty(e, 'returnValue', { get: () => undefined, set: () => {}, configurable: true })
        },
        { capture: true },
      )
    } catch {
      // cross-origin frame — can't suppress, but this shouldn't happen
    }
  }
  activeTab.value = name
}

async function handleRestart(e: MouseEvent, viewerName: string, displayName: string) {
  if (!e.shiftKey && !confirm(`Restart ${displayName}?`)) return
  await $fetch(`/api/workspaces/${wsName}/viewers/${viewerName}/restart`, { method: 'POST' })
}

function formatCpu(mc: number): string {
  if (mc === 0) return '—'
  return mc >= 1000 ? `${(mc / 1000).toFixed(1)}` : `${mc}m`
}

function formatMemory(mi: number): string {
  if (mi === 0) return '—'
  return mi >= 1024 ? `${(mi / 1024).toFixed(1)} Gi` : `${mi} Mi`
}
</script>

<template>
  <div class="shell">
    <!-- Left sidebar -->
    <nav class="sidebar">
      <div class="sidebar-top">
        <NuxtLink :to="`/projects/${project}`" class="back-link">← {{ project }}</NuxtLink>
        <span class="ws-label">{{ wsName }}</span>
      </div>

      <ul class="nav-list">
        <li>
          <button
            :class="['nav-item', { active: activeTab === 'dashboard' }]"
            @click="switchViewer('dashboard')"
          >
            Dashboard
          </button>
        </li>
        <li v-if="workspace?.viewers.length" class="nav-section-label">Viewers</li>
        <li v-for="viewer in workspace?.viewers ?? []" :key="viewer.name" class="nav-item-row">
          <button
            :class="['nav-item', { active: activeTab === viewer.name }]"
            :disabled="workspace?.status !== 'ready'"
            @click="switchViewer(viewer.name)"
          >
            {{ viewer.display_name }}
          </button>
          <button
            class="restart-btn"
            :disabled="workspace?.status !== 'ready'"
            :title="`Restart ${viewer.display_name} (shift-click to skip confirm)`"
            @click="handleRestart($event, viewer.name, viewer.display_name)"
          >
            ↺
          </button>
        </li>
      </ul>
    </nav>

    <!-- Main content -->
    <main class="content">
      <!-- Viewer iframe -->
      <iframe
        v-if="activeTab !== 'dashboard' && activeViewer"
        ref="iframeRef"
        :src="`${activeViewer.path}/`"
        class="viewer-frame"
        frameborder="0"
        allow="fullscreen"
      />

      <!-- Dashboard -->
      <div v-else class="dashboard-scroll">
      <div class="dashboard">
        <div v-if="wsError" class="error-banner">{{ wsError }}</div>

        <h1 class="dash-title">{{ wsName }}</h1>

        <div class="stats-grid">
          <div class="stat-card">
            <span class="stat-label">Status</span>
            <span :class="['stat-value', 'status-badge', `status-${workspace?.status ?? 'pending'}`]">
              {{ workspace?.status === 'pending' || workspace?.status === 'provisioning' ? 'connecting' : workspace?.status ?? '...' }}
            </span>
          </div>

          <div class="stat-card">
            <span class="stat-label">Pods</span>
            <span class="stat-value">{{ metrics?.pod_count ?? '—' }}</span>
          </div>

          <div class="stat-card">
            <span class="stat-label">CPU Requests</span>
            <span class="stat-value">{{ metrics ? formatCpu(metrics.cpu_requests_millicores) : '—' }}</span>
          </div>

          <div class="stat-card">
            <span class="stat-label">CPU Limits</span>
            <span class="stat-value">{{ metrics ? formatCpu(metrics.cpu_limits_millicores) : '—' }}</span>
          </div>

          <div class="stat-card">
            <span class="stat-label">Memory Requests</span>
            <span class="stat-value">{{ metrics ? formatMemory(metrics.memory_requests_mi) : '—' }}</span>
          </div>

          <div class="stat-card">
            <span class="stat-label">Memory Limits</span>
            <span class="stat-value">{{ metrics ? formatMemory(metrics.memory_limits_mi) : '—' }}</span>
          </div>
        </div>

        <div class="detail-section">
          <h2 class="section-title">Repositories</h2>
          <ul class="repo-list">
            <li v-for="repo in workspace?.repos ?? []" :key="repo" class="repo-item">
              {{ repo }}
            </li>
          </ul>
        </div>

        <div class="detail-section">
          <h2 class="section-title">Namespace</h2>
          <code class="ns-badge">{{ workspace?.namespace ?? '...' }}</code>
        </div>
      </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

/* ── Sidebar ──────────────────────────────────────────────── */

.sidebar {
  width: 200px;
  flex-shrink: 0;
  background: var(--surface);
  border-right: 1px solid var(--border-dim);
  display: flex;
  flex-direction: column;
  padding: 1rem 0;
}

.sidebar-top {
  padding: 0 1rem 1rem;
  border-bottom: 1px solid var(--border-dim);
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.back-link {
  font-size: 0.75rem;
  color: var(--text-dim);
  text-decoration: none;
  transition: color 0.12s;
}

.back-link:hover {
  color: var(--accent);
}

.ws-label {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--heading);
  word-break: break-all;
}

.nav-list {
  list-style: none;
  padding: 0.75rem 0;
  margin: 0;
  flex: 1;
}

.nav-section-label {
  font-size: 0.65rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-dim);
  padding: 0.75rem 1rem 0.25rem;
}

.nav-item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 0.5rem 1rem;
  font-family: inherit;
  font-size: 0.82rem;
  color: var(--text);
  background: none;
  border: none;
  cursor: pointer;
  transition: all 0.12s;
}

.nav-item:hover:not(:disabled) {
  background: rgba(139, 96, 60, 0.15);
  color: var(--accent);
}

.nav-item.active {
  color: var(--accent-bright);
  background: rgba(139, 96, 60, 0.2);
  border-left: 2px solid var(--accent);
}

.nav-item:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.nav-item-row {
  display: flex;
  align-items: center;
}

.nav-item-row .nav-item {
  flex: 1;
}

.restart-btn {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 0.95rem;
  padding: 0.35rem 0.5rem;
  cursor: pointer;
  transition: color 0.12s;
  line-height: 1;
}

.restart-btn:hover:not(:disabled) {
  color: var(--accent);
}

.restart-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* ── Main content ─────────────────────────────────────────── */

.content {
  flex: 1;
  overflow: hidden;
  background: var(--bg);
}

.dashboard-scroll {
  height: 100%;
  overflow-y: auto;
}

.viewer-frame {
  width: 100%;
  height: 100%;
  border: none;
  display: block;
}

/* ── Dashboard ────────────────────────────────────────────── */

.dashboard {
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem 2.5rem 4rem;
}

.error-banner {
  background: var(--danger-bg);
  border: 1px solid var(--danger);
  border-left: 3px solid var(--danger);
  color: #dd8888;
  padding: 0.65rem 1rem;
  margin-bottom: 1.5rem;
  font-size: 0.85rem;
}

.dash-title {
  font-family: 'Cinzel', 'Palatino Linotype', Palatino, serif;
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--heading);
  letter-spacing: 0.12em;
  margin-bottom: 1.75rem;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  gap: 0.85rem;
  margin-bottom: 2rem;
}

.stat-card {
  background: var(--surface);
  border: 1px solid var(--border-dim);
  padding: 1rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.stat-label {
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-dim);
}

.stat-value {
  font-size: 1.15rem;
  font-weight: 600;
  color: var(--heading);
}

/* reuse status badge styles from WorkspaceList */
.status-badge {
  display: inline-block;
  font-size: 0.72rem;
  letter-spacing: 0.06em;
  text-transform: capitalize;
  padding: 0.2rem 0.6rem;
  border: 1px solid;
  font-weight: 500;
}

.status-pending,
.status-provisioning {
  color: var(--accent);
  border-color: rgba(204, 136, 80, 0.4);
  background: rgba(204, 136, 80, 0.1);
}

.status-ready {
  color: var(--ready);
  border-color: rgba(90, 184, 112, 0.38);
  background: var(--ready-bg);
}

.status-failed {
  color: var(--danger);
  border-color: rgba(200, 68, 68, 0.38);
  background: var(--danger-bg);
}

.detail-section {
  margin-bottom: 1.5rem;
}

.section-title {
  font-size: 0.78rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent);
  margin-bottom: 0.6rem;
  padding-bottom: 0.4rem;
  border-bottom: 1px solid var(--border-dim);
}

.repo-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.repo-item {
  font-size: 0.82rem;
  color: var(--text-dim);
  font-family: 'Courier New', Courier, monospace;
}

.ns-badge {
  font-size: 0.82rem;
  color: var(--text-dim);
  font-family: 'Courier New', Courier, monospace;
}
</style>
