import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useEnvironments } from '../../../composables/useEnvironments'

vi.mock('../../../api/environments', () => ({
  fetchEnvironments: vi.fn(),
  createEnvironment: vi.fn(),
  deleteEnvironment: vi.fn(),
}))

import {
  fetchEnvironments,
  createEnvironment,
  deleteEnvironment,
} from '../../../api/environments'

const mockEnv = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  name: 'test-env',
  repos: ['https://github.com/foo/bar'],
  namespace: 'env-test-env',
  status: 'pending' as const,
  created_at: '2024-01-01T00:00:00Z',
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useEnvironments', () => {
  describe('load', () => {
    it('sets environments on success', async () => {
      vi.mocked(fetchEnvironments).mockResolvedValue([mockEnv])
      const { environments, pending, error, load } = useEnvironments()
      await load()
      expect(environments.value).toHaveLength(1)
      expect(environments.value[0].name).toBe('test-env')
      expect(pending.value).toBe(false)
      expect(error.value).toBeNull()
    })

    it('sets error on failure', async () => {
      vi.mocked(fetchEnvironments).mockRejectedValue(new Error('network error'))
      const { environments, pending, error, load } = useEnvironments()
      await load()
      expect(error.value?.message).toBe('network error')
      expect(pending.value).toBe(false)
      expect(environments.value).toHaveLength(0)
    })
  })

  describe('create', () => {
    it('appends environment to list', async () => {
      vi.mocked(createEnvironment).mockResolvedValue(mockEnv)
      const { environments, create } = useEnvironments()
      await create({ name: 'test-env', repos: ['https://github.com/foo/bar'] })
      expect(environments.value).toHaveLength(1)
      expect(environments.value[0].name).toBe('test-env')
    })
  })

  describe('remove', () => {
    it('removes environment from list', async () => {
      vi.mocked(fetchEnvironments).mockResolvedValue([mockEnv])
      vi.mocked(deleteEnvironment).mockResolvedValue(undefined)
      const { environments, load, remove } = useEnvironments()
      await load()
      expect(environments.value).toHaveLength(1)
      await remove('test-env')
      expect(environments.value).toHaveLength(0)
    })
  })
})
