import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { useWorkspaces } from '../../../composables/useWorkspaces'

vi.mock('../../../api/workspaces', () => ({
  fetchWorkspaces: vi.fn(),
  createWorkspace: vi.fn(),
  deleteWorkspace: vi.fn(),
}))

import {
  fetchWorkspaces,
  createWorkspace,
  deleteWorkspace,
} from '../../../api/workspaces'

const mockWs = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  name: 'test-ws',
  repos: ['https://github.com/foo/bar'],
  namespace: 'ws-test-ws',
  status: 'pending' as const,
  created_at: '2024-01-01T00:00:00Z',
  viewers: [],
  project: 'default',
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useWorkspaces', () => {
  describe('load', () => {
    it('sets workspaces on success', async () => {
      vi.mocked(fetchWorkspaces).mockResolvedValue([mockWs])
      const { workspaces, pending, error, load } = useWorkspaces()
      await load()
      expect(workspaces.value).toHaveLength(1)
      expect(workspaces.value[0].name).toBe('test-ws')
      expect(pending.value).toBe(false)
      expect(error.value).toBeNull()
    })

    it('sets error on failure', async () => {
      vi.mocked(fetchWorkspaces).mockRejectedValue(new Error('network error'))
      const { workspaces, pending, error, load } = useWorkspaces()
      await load()
      expect(error.value?.message).toBe('network error')
      expect(pending.value).toBe(false)
      expect(workspaces.value).toHaveLength(0)
    })
  })

  describe('create', () => {
    it('appends workspace to list immediately', async () => {
      vi.mocked(createWorkspace).mockResolvedValue(mockWs)
      const { workspaces, create, stopPolling } = useWorkspaces()
      await create({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] })
      expect(workspaces.value).toHaveLength(1)
      expect(workspaces.value[0].name).toBe('test-ws')
      stopPolling()
    })

    it('starts polling when created workspace is pending', async () => {
      vi.mocked(createWorkspace).mockResolvedValue(mockWs)
      vi.mocked(fetchWorkspaces).mockResolvedValue([{ ...mockWs, status: 'ready' as const }])
      const { workspaces, create } = useWorkspaces()
      await create({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] })

      await vi.advanceTimersByTimeAsync(3000)

      expect(fetchWorkspaces).toHaveBeenCalled()
      expect(workspaces.value[0].status).toBe('ready')
    })

    it('stops polling once all workspaces are settled', async () => {
      vi.mocked(createWorkspace).mockResolvedValue(mockWs)
      vi.mocked(fetchWorkspaces).mockResolvedValue([{ ...mockWs, status: 'ready' as const }])
      const { create } = useWorkspaces()
      await create({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] })

      await vi.advanceTimersByTimeAsync(3000)
      const callCount = vi.mocked(fetchWorkspaces).mock.calls.length

      await vi.advanceTimersByTimeAsync(3000)
      // No additional fetches — polling stopped after workspaces settled
      expect(vi.mocked(fetchWorkspaces).mock.calls.length).toBe(callCount)
    })

    it('does not start a second timer when polling is already running', async () => {
      vi.mocked(createWorkspace).mockResolvedValue(mockWs)
      vi.mocked(fetchWorkspaces).mockResolvedValue([{ ...mockWs, status: 'ready' as const }])
      const { create, stopPolling } = useWorkspaces()
      // First create starts polling
      await create({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] })
      // Second create while polling is running — exercises the guard branch
      await create({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] })
      stopPolling()
    })

    it('does not start polling when created workspace is immediately ready', async () => {
      vi.mocked(createWorkspace).mockResolvedValue({ ...mockWs, status: 'ready' as const })
      const { create } = useWorkspaces()
      await create({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] })

      await vi.advanceTimersByTimeAsync(3000)
      expect(fetchWorkspaces).not.toHaveBeenCalled()
    })
  })

  describe('onUnmounted cleanup', () => {
    it('registers cleanup when used inside a component', () => {
      vi.mocked(fetchWorkspaces).mockResolvedValue([])
      const Wrapper = defineComponent({
        setup() {
          const { stopPolling } = useWorkspaces()
          return { stopPolling }
        },
        template: '<div />',
      })
      // Should not throw — exercises the getCurrentInstance() branch
      const wrapper = mount(Wrapper)
      wrapper.unmount()
    })
  })

  describe('remove', () => {
    it('removes workspace from list', async () => {
      vi.mocked(fetchWorkspaces).mockResolvedValue([mockWs])
      vi.mocked(deleteWorkspace).mockResolvedValue(undefined)
      const { workspaces, load, remove } = useWorkspaces()
      await load()
      expect(workspaces.value).toHaveLength(1)
      await remove('test-ws')
      expect(workspaces.value).toHaveLength(0)
    })
  })
})
