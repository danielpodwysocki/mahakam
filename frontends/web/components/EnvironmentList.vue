<script setup lang="ts">
import type { Environment } from '../api/environments'

defineProps<{
  environments: Environment[]
  onDelete: (name: string) => void
}>()
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Namespace</th>
        <th>Status</th>
        <th>Repos</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="env in environments" :key="env.id">
        <td>{{ env.name }}</td>
        <td>{{ env.namespace }}</td>
        <td>{{ env.status }}</td>
        <td>{{ env.repos.join(', ') }}</td>
        <td>
          <button @click="onDelete(env.name)">Delete</button>
        </td>
      </tr>
    </tbody>
  </table>
</template>
