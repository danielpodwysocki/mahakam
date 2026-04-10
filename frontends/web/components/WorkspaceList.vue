<script setup lang="ts">
import type { Workspace } from '../api/workspaces'

defineProps<{
  workspaces: Workspace[]
  onDelete: (name: string) => void
  project: string
}>()

function statusLabel(status: string): string {
  return status === 'pending' || status === 'provisioning' ? 'connecting' : status
}

function isSettled(status: string): boolean {
  return status === 'ready' || status === 'failed'
}
</script>

<template>
  <div>
    <h2 class="section-title">Workspaces</h2>
    <p v-if="workspaces.length === 0" class="empty">
      No workspaces yet.
    </p>
    <table v-else class="ws-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Namespace</th>
          <th>Status</th>
          <th>Repositories</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="ws in workspaces" :key="ws.id">
          <td class="td-name">{{ ws.name }}</td>
          <td class="td-ns">{{ ws.namespace }}</td>
          <td>
            <span :class="['status-badge', `status-${ws.status}`]">{{ statusLabel(ws.status) }}</span>
          </td>
          <td class="td-repos">{{ ws.repos.join(', ') }}</td>
          <td class="td-actions">
            <NuxtLink
              :to="`/projects/${project}/workspaces/${ws.name}`"
              class="btn-open"
            >Open</NuxtLink>
            <button
              class="btn-delete"
              :disabled="!isSettled(ws.status)"
              @click="onDelete(ws.name)"
            >Delete</button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.section-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--accent);
  margin-bottom: 1.2rem;
  padding-bottom: 0.55rem;
  border-bottom: 1px solid var(--border-dim);
}

.empty {
  text-align: center;
  color: var(--text-dim);
  font-size: 0.88rem;
  padding: 2rem 0;
}

.ws-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.88rem;
}

.ws-table thead tr {
  background: var(--input-bg);
}

.ws-table th {
  text-align: left;
  font-size: 0.78rem;
  color: var(--heading);
  padding: 0.6rem 0.75rem;
  border-bottom: 2px solid var(--border-dim);
  font-weight: 600;
}

.ws-table tbody tr {
  border-bottom: 1px solid rgba(139, 96, 64, 0.22);
  transition: background 0.12s;
}

.ws-table tbody tr:last-child {
  border-bottom: none;
}

.ws-table tbody tr:hover {
  background: rgba(62, 44, 30, 0.5);
}

.ws-table td {
  padding: 0.65rem 0.75rem;
  vertical-align: middle;
}

.td-name {
  font-weight: 600;
  color: var(--heading);
}

.td-ns {
  color: var(--text-dim);
  font-size: 0.82rem;
  font-family: 'Courier New', Courier, monospace;
}

.td-repos {
  font-size: 0.78rem;
  color: var(--text-dim);
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-badge {
  display: inline-block;
  font-size: 0.68rem;
  letter-spacing: 0.06em;
  text-transform: capitalize;
  padding: 0.2rem 0.6rem;
  border: 1px solid;
  font-weight: 500;
}

.status-pending {
  color: var(--accent);
  border-color: rgba(204, 136, 80, 0.4);
  background: rgba(204, 136, 80, 0.1);
}

.status-provisioning {
  color: var(--accent-bright);
  border-color: rgba(232, 168, 64, 0.4);
  background: rgba(232, 168, 64, 0.1);
  animation: pulse-dim 1.5s ease-in-out infinite;
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

.td-actions {
  display: flex;
  gap: 0.4rem;
  align-items: center;
}

.btn-open {
  font-family: inherit;
  font-size: 0.72rem;
  padding: 0.28rem 0.65rem;
  background: transparent;
  border: 1px solid rgba(139, 96, 60, 0.4);
  color: var(--accent);
  cursor: pointer;
  text-decoration: none;
  display: inline-block;
  transition: all 0.15s;
}

.btn-open:hover {
  background: rgba(139, 96, 60, 0.15);
  border-color: var(--accent);
}

.btn-delete {
  font-family: inherit;
  font-size: 0.72rem;
  padding: 0.28rem 0.65rem;
  background: transparent;
  border: 1px solid rgba(200, 68, 68, 0.4);
  color: var(--danger);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-delete:hover:not(:disabled) {
  background: var(--danger-bg);
  border-color: var(--danger);
}

.btn-delete:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
</style>
