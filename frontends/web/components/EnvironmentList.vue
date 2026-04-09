<script setup lang="ts">
import { ref } from 'vue'
import type { Environment } from '../api/environments'
import TerminalViewer from './TerminalViewer.vue'

defineProps<{
  environments: Environment[]
  onDelete: (name: string) => void
}>()

function statusLabel(status: string): string {
  return status === 'pending' || status === 'provisioning' ? 'connecting' : status
}

function isSettled(status: string): boolean {
  return status === 'ready' || status === 'failed'
}

const activeConsole = ref<string | null>(null)
function openConsole(name: string) { activeConsole.value = name }
function closeConsole() { activeConsole.value = null }
</script>

<template>
  <div>
    <h2 class="section-title">Environments</h2>
    <p v-if="environments.length === 0" class="empty">
      No environments yet.
    </p>
    <table v-else class="env-table">
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
        <tr v-for="env in environments" :key="env.id">
          <td class="td-name">{{ env.name }}</td>
          <td class="td-ns">{{ env.namespace }}</td>
          <td>
            <span :class="['status-badge', `status-${env.status}`]">{{ statusLabel(env.status) }}</span>
          </td>
          <td class="td-repos">{{ env.repos.join(', ') }}</td>
          <td class="td-actions">
            <button
              class="btn-delete"
              :disabled="!isSettled(env.status)"
              @click="onDelete(env.name)"
            >Delete</button>
            <button
              class="btn-console"
              :disabled="env.status !== 'ready'"
              @click="openConsole(env.name)"
            >Open Console</button>
          </td>
        </tr>
      </tbody>
    </table>

    <TerminalViewer
      v-if="activeConsole"
      :env-name="activeConsole"
      @close="closeConsole"
    />
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

.env-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.88rem;
}

.env-table thead tr {
  background: var(--input-bg);
}

.env-table th {
  text-align: left;
  font-size: 0.78rem;
  color: var(--heading);
  padding: 0.6rem 0.75rem;
  border-bottom: 2px solid var(--border-dim);
  font-weight: 600;
}

.env-table tbody tr {
  border-bottom: 1px solid rgba(139, 96, 64, 0.22);
  transition: background 0.12s;
}

.env-table tbody tr:last-child {
  border-bottom: none;
}

.env-table tbody tr:hover {
  background: rgba(62, 44, 30, 0.5);
}

.env-table td {
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

.td-actions {
  display: flex;
  gap: 0.4rem;
  align-items: center;
}

.btn-console {
  font-family: inherit;
  font-size: 0.72rem;
  padding: 0.28rem 0.65rem;
  background: transparent;
  border: 1px solid rgba(139, 96, 60, 0.4);
  color: var(--accent);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-console:hover:not(:disabled) {
  background: rgba(139, 96, 60, 0.15);
  border-color: var(--accent);
}

.btn-console:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
</style>
