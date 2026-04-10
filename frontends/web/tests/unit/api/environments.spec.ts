import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fetchEnvironments, createEnvironment, deleteEnvironment } from '../../../api/environments'

const mockEnv = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  name: 'test-env',
  repos: ['https://github.com/foo/bar'],
  namespace: 'env-test-env',
  status: 'pending' as const,
  created_at: '2024-01-01T00:00:00Z',
  viewers: [],
}

beforeEach(() => {
  vi.restoreAllMocks()
})

describe('fetchEnvironments', () => {
  it('fetches and parses environments', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve([mockEnv]),
      }),
    )
    const result = await fetchEnvironments()
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('test-env')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
      }),
    )
    await expect(fetchEnvironments()).rejects.toThrow('HTTP 500')
  })
})

describe('createEnvironment', () => {
  it('creates and returns environment', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockEnv),
      }),
    )
    const result = await createEnvironment({
      name: 'test-env',
      repos: ['https://github.com/foo/bar'],
    })
    expect(result.name).toBe('test-env')
    expect(result.namespace).toBe('env-test-env')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 422,
      }),
    )
    await expect(
      createEnvironment({ name: 'test-env', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] }),
    ).rejects.toThrow('HTTP 422')
  })
})

describe('deleteEnvironment', () => {
  it('deletes successfully on 204', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
      }),
    )
    await expect(deleteEnvironment('test-env')).resolves.toBeUndefined()
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
      }),
    )
    await expect(deleteEnvironment('test-env')).rejects.toThrow('HTTP 404')
  })
})
