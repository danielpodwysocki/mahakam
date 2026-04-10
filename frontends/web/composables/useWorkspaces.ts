import { ref, getCurrentInstance, onUnmounted } from 'vue'
import {
  fetchWorkspaces,
  createWorkspace,
  deleteWorkspace,
  type Workspace,
  type CreateWorkspace,
} from '../api/workspaces'

const POLL_INTERVAL_MS = 3000

function isSettled(ws: Workspace): boolean {
  return ws.status === 'ready' || ws.status === 'failed'
}

/** Manages workspace state and API interactions. */
export function useWorkspaces() {
  const workspaces = ref<Workspace[]>([])
  const pending = ref(false)
  const error = ref<Error | null>(null)
  let pollingTimer: ReturnType<typeof setInterval> | null = null

  /** Loads all workspaces from the API. */
  async function load(): Promise<void> {
    pending.value = true
    error.value = null
    try {
      workspaces.value = await fetchWorkspaces()
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
      if (workspaces.value.every(isSettled)) {
        stopPolling()
      }
    }, POLL_INTERVAL_MS)
  }

  // Clean up the timer when the owning component unmounts.
  if (getCurrentInstance()) {
    onUnmounted(stopPolling)
  }

  /** Creates a workspace, appends it immediately with pending status, then polls until settled. */
  async function create(data: CreateWorkspace): Promise<void> {
    const ws = await createWorkspace(data)
    workspaces.value = [...workspaces.value, ws]
    if (!isSettled(ws)) {
      startPolling()
    }
  }

  /** Removes a workspace by name. */
  async function remove(name: string): Promise<void> {
    await deleteWorkspace(name)
    workspaces.value = workspaces.value.filter((w) => w.name !== name)
  }

  return { workspaces, pending, error, load, create, remove, stopPolling }
}
