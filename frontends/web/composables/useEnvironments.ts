import { ref, getCurrentInstance, onUnmounted } from 'vue'
import {
  fetchEnvironments,
  createEnvironment,
  deleteEnvironment,
  type Environment,
  type CreateEnvironment,
} from '../api/environments'

const POLL_INTERVAL_MS = 3000

function isSettled(env: Environment): boolean {
  return env.status === 'ready' || env.status === 'failed'
}

/** Manages environment state and API interactions. */
export function useEnvironments() {
  const environments = ref<Environment[]>([])
  const pending = ref(false)
  const error = ref<Error | null>(null)
  let pollingTimer: ReturnType<typeof setInterval> | null = null

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

  function stopPolling(): void {
    if (pollingTimer !== null) {
      clearInterval(pollingTimer)
      pollingTimer = null
    }
  }

  function startPolling(): void {
    if (pollingTimer !== null) return
    pollingTimer = setInterval(async () => {
      await load()
      if (environments.value.every(isSettled)) {
        stopPolling()
      }
    }, POLL_INTERVAL_MS)
  }

  // Clean up the timer when the owning component unmounts.
  if (getCurrentInstance()) {
    onUnmounted(stopPolling)
  }

  /** Creates an environment, appends it immediately with pending status, then polls until settled. */
  async function create(data: CreateEnvironment): Promise<void> {
    const env = await createEnvironment(data)
    environments.value = [...environments.value, env]
    if (!isSettled(env)) {
      startPolling()
    }
  }

  /** Removes an environment by name. */
  async function remove(name: string): Promise<void> {
    await deleteEnvironment(name)
    environments.value = environments.value.filter((e) => e.name !== name)
  }

  return { environments, pending, error, load, create, remove, stopPolling }
}
