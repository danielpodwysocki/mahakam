<script setup lang="ts">
import { ref, reactive } from 'vue'
import { CreateEnvironmentSchema, type CreateEnvironment } from '../api/environments'

const props = defineProps<{
  onSubmit: (data: CreateEnvironment) => void
  submitting: boolean
}>()

const name = ref('')
const repos = ref<string[]>([''])
const errors = reactive<{ name?: string; repos?: string }>({})

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
    <div>
      <label for="env-name">Name</label>
      <input id="env-name" v-model="name" type="text" placeholder="my-environment" />
      <span v-if="errors.name" class="error">{{ errors.name }}</span>
    </div>
    <div v-for="(repo, i) in repos" :key="i">
      <input
        :value="repo"
        type="url"
        placeholder="https://github.com/org/repo"
        @input="updateRepo(i, ($event.target as HTMLInputElement).value)"
      />
      <button type="button" :disabled="repos.length <= 1" @click="removeRepo(i)">Remove</button>
    </div>
    <button type="button" @click="addRepo">Add Repository</button>
    <span v-if="errors.repos" class="error">{{ errors.repos }}</span>
    <div>
      <button type="submit" :disabled="submitting">
        {{ submitting ? 'Creating...' : 'Create Environment' }}
      </button>
    </div>
  </form>
</template>
