import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fetchWorkspaces, createWorkspace, deleteWorkspace, fetchWorkspace, fetchWorkspaceMetrics } from '../../../api/workspaces'

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
  vi.restoreAllMocks()
})

describe('fetchWorkspaces', () => {
  it('fetches and parses workspaces', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve([mockWs]),
      }),
    )
    const result = await fetchWorkspaces()
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('test-ws')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
      }),
    )
    await expect(fetchWorkspaces()).rejects.toThrow('HTTP 500')
  })
})

describe('createWorkspace', () => {
  it('creates and returns workspace', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockWs),
      }),
    )
    const result = await createWorkspace({
      name: 'test-ws',
      repos: ['https://github.com/foo/bar'],
      viewers: ['terminal'],
    })
    expect(result.name).toBe('test-ws')
    expect(result.namespace).toBe('ws-test-ws')
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
      createWorkspace({ name: 'test-ws', repos: ['https://github.com/foo/bar'], viewers: ['terminal'] }),
    ).rejects.toThrow('HTTP 422')
  })
})

describe('fetchWorkspace', () => {
  it('fetches and parses a single workspace', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockWs),
      }),
    )
    const result = await fetchWorkspace('test-ws')
    expect(result.name).toBe('test-ws')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: false, status: 404 }),
    )
    await expect(fetchWorkspace('missing')).rejects.toThrow('HTTP 404')
  })
})

describe('fetchWorkspaceMetrics', () => {
  it('fetches and parses workspace metrics', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            pod_count: 3,
            cpu_requests_millicores: 350,
            cpu_limits_millicores: 1000,
            memory_requests_mi: 512,
            memory_limits_mi: 1024,
          }),
      }),
    )
    const result = await fetchWorkspaceMetrics('test-ws')
    expect(result.pod_count).toBe(3)
    expect(result.cpu_requests_millicores).toBe(350)
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: false, status: 500 }),
    )
    await expect(fetchWorkspaceMetrics('test-ws')).rejects.toThrow('HTTP 500')
  })
})

describe('deleteWorkspace', () => {
  it('deletes successfully on 204', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
      }),
    )
    await expect(deleteWorkspace('test-ws')).resolves.toBeUndefined()
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
      }),
    )
    await expect(deleteWorkspace('test-ws')).rejects.toThrow('HTTP 404')
  })
})
