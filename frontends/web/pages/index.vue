<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useEnvironments } from '../composables/useEnvironments'
import EnvironmentForm from '../components/EnvironmentForm.vue'
import EnvironmentList from '../components/EnvironmentList.vue'
import type { CreateEnvironment } from '../api/environments'

const { environments, pending, error, load, create, remove } = useEnvironments()
const submitting = ref(false)

onMounted(async () => {
  await load()
})

async function handleCreate(data: CreateEnvironment) {
  submitting.value = true
  try {
    await create(data)
  } finally {
    submitting.value = false
  }
}

async function handleDelete(name: string) {
  await remove(name)
}
</script>

<template>
  <div>
    <h1>Mahakam Environments</h1>
    <EnvironmentForm :on-submit="handleCreate" :submitting="submitting" />
    <div v-if="pending">Loading...</div>
    <div v-else-if="error">Error: {{ error.message }}</div>
    <EnvironmentList v-else :environments="environments" :on-delete="handleDelete" />
  </div>
</template>
