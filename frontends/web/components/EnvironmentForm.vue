<script setup lang="ts">
import { ref, reactive } from 'vue'
import { CreateEnvironmentSchema, type CreateEnvironment } from '../api/environments'

const props = defineProps<{
  onSubmit: (data: CreateEnvironment) => void
  submitting: boolean
}>()

const name = ref('')
const repos = ref<string[]>([''])
const selectedViewers = ref<string[]>(['terminal'])
const errors = reactive<{ name?: string; repos?: string }>({})

const AVAILABLE_VIEWERS = [
  { name: 'terminal', label: 'Terminal' },
  { name: 'browser', label: 'Browser' },
]

function isViewerSelected(viewerName: string): boolean {
  return selectedViewers.value.includes(viewerName)
}

function toggleViewer(viewerName: string, checked: boolean) {
  if (checked) {
    selectedViewers.value = [...selectedViewers.value, viewerName]
  } else {
    selectedViewers.value = selectedViewers.value.filter((v) => v !== viewerName)
  }
}

function addRepo() {
  repos.value = [...repos.value, '']
}

function removeRepo(index: number) {
  repos.value = repos.value.filter((_, i) => i !== index)
}

function updateRepo(index: number, value: string) {
  const next = [...repos.value]
  next[index] = value
  repos.value = next
}

function handleSubmit() {
  const data = {
    name: name.value,
    repos: repos.value.filter((r) => r.trim() !== ''),
    viewers: selectedViewers.value,
  }
  const result = CreateEnvironmentSchema.safeParse(data)
  if (!result.success) {
    const fieldErrors = result.error.flatten().fieldErrors
    errors.name = fieldErrors.name?.[0]
    errors.repos = fieldErrors.repos?.[0]
    return
  }
  errors.name = undefined
  errors.repos = undefined
  props.onSubmit(result.data)
}
</script>

<template>
  <form @submit.prevent="handleSubmit">
    <h2 class="section-title">New Workspace</h2>

    <div class="field">
      <label class="label" for="env-name">Name</label>
      <input
        id="env-name"
        v-model="name"
        class="input"
        type="text"
        placeholder="my-environment"
        autocomplete="off"
        spellcheck="false"
      />
      <span v-if="errors.name" class="error">{{ errors.name }}</span>
    </div>

    <div class="field">
      <label class="label">Repositories</label>
      <div v-for="(repo, i) in repos" :key="i" class="repo-row">
        <input
          :value="repo"
          class="input"
          type="url"
          placeholder="https://github.com/org/repo"
          @input="updateRepo(i, ($event.target as HTMLInputElement).value)"
        />
        <button
          type="button"
          class="btn btn-ghost-danger"
          :disabled="repos.length <= 1"
          @click="removeRepo(i)"
        >Remove</button>
      </div>
      <span v-if="errors.repos" class="error">{{ errors.repos }}</span>
    </div>

    <div class="field">
      <label class="label">Viewers</label>
      <div class="viewer-options">
        <label
          v-for="viewer in AVAILABLE_VIEWERS"
          :key="viewer.name"
          class="viewer-option"
        >
          <input
            type="checkbox"
            :value="viewer.name"
            :checked="isViewerSelected(viewer.name)"
            @change="toggleViewer(viewer.name, ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ viewer.label }}</span>
        </label>
      </div>
    </div>

    <div class="form-footer">
      <button type="button" class="btn btn-secondary" @click="addRepo">
        + Add Repository
      </button>
      <button type="submit" class="btn btn-primary" :disabled="submitting">
        {{ submitting ? 'Creating...' : 'Create' }}
      </button>
    </div>
  </form>
</template>

<style scoped>
.section-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--accent);
  margin-bottom: 1.4rem;
  padding-bottom: 0.55rem;
  border-bottom: 1px solid var(--border-dim);
}

.field {
  margin-bottom: 1.2rem;
}

.label {
  display: block;
  font-size: 0.8rem;
  color: var(--heading);
  margin-bottom: 0.4rem;
}

.input {
  width: 100%;
  background: var(--input-bg);
  border: 1px solid var(--border-dim);
  color: var(--text);
  font-family: inherit;
  font-size: 0.9rem;
  padding: 0.5rem 0.75rem;
  outline: none;
  transition: border-color 0.18s, box-shadow 0.18s;
}

.input::placeholder {
  color: var(--text-muted);
  font-style: italic;
}

.input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(204, 136, 80, 0.2);
}

.repo-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.4rem;
  align-items: center;
}

.repo-row .input {
  flex: 1;
}

.viewer-options {
  display: flex;
  gap: 1.2rem;
}

.viewer-option {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.85rem;
  color: var(--text);
  cursor: pointer;
  user-select: none;
}

.viewer-option input[type="checkbox"] {
  accent-color: var(--accent);
  width: 0.9rem;
  height: 0.9rem;
  cursor: pointer;
}

.error {
  display: block;
  font-size: 0.74rem;
  color: var(--danger);
  margin-top: 0.3rem;
}

.form-footer {
  display: flex;
  gap: 0.75rem;
  justify-content: flex-end;
  margin-top: 1.4rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-dim);
}

.btn {
  font-family: inherit;
  font-size: 0.78rem;
  letter-spacing: 0.06em;
  padding: 0.48rem 1.1rem;
  border: 1px solid;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-primary {
  background: var(--accent);
  border-color: var(--accent-bright);
  color: #1a0e08;
  font-weight: 600;
}

.btn-primary:hover:not(:disabled) {
  background: var(--accent-bright);
  box-shadow: 0 0 10px rgba(232, 168, 64, 0.3);
}

.btn-secondary {
  background: transparent;
  border-color: var(--border-dim);
  color: var(--accent);
}

.btn-secondary:hover:not(:disabled) {
  border-color: var(--accent);
  background: rgba(204, 136, 80, 0.08);
}

.btn-ghost-danger {
  background: transparent;
  border-color: rgba(200, 68, 68, 0.45);
  color: var(--danger);
  font-size: 0.72rem;
  padding: 0.48rem 0.7rem;
  flex-shrink: 0;
}

.btn-ghost-danger:hover:not(:disabled) {
  background: var(--danger-bg);
  border-color: var(--danger);
}
</style>
