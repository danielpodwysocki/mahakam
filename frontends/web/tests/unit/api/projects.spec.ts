import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fetchProjects } from '../../../api/projects'

beforeEach(() => {
  vi.restoreAllMocks()
})

describe('fetchProjects', () => {
  it('fetches and parses projects', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve([{ name: 'default', workspace_count: 2 }]),
      }),
    )
    const result = await fetchProjects()
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('default')
    expect(result[0].workspace_count).toBe(2)
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
      }),
    )
    await expect(fetchProjects()).rejects.toThrow('HTTP 500')
  })
})
