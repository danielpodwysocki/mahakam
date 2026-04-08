import { ref } from 'vue'
import {
  fetchEnvironments,
  createEnvironment,
  deleteEnvironment,
  type Environment,
  type CreateEnvironment,
} from '../api/environments'

/** Manages environment state and API interactions. */
export function useEnvironments() {
  const environments = ref<Environment[]>([])
  const pending = ref(false)
  const error = ref<Error | null>(null)

  /** Loads all environments from the API. */
  async function load(): Promise<void> {
    pending.value = true
    error.value = null
    try {
      environments.value = await fetchEnvironments()
    } catch (e) {
      error.value = e as Error
    } finally {
      pending.value = false
    }
  }

  /** Creates an environment and appends it to the local list. */
  async function create(data: CreateEnvironment): Promise<void> {
    const env = await createEnvironment(data)
    environments.value = [...environments.value, env]
  }

  /** Removes an environment by name. */
  async function remove(name: string): Promise<void> {
    await deleteEnvironment(name)
    environments.value = environments.value.filter((e) => e.name !== name)
  }

  return { environments, pending, error, load, create, remove }
}
