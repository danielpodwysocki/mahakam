import { z } from 'zod'

export const ViewerSchema = z.object({
  name: z.string(),
  display_name: z.string(),
  path: z.string(),
})

export const WorkspaceSchema = z.object({
  id: z.string(),
  name: z.string(),
  repos: z.array(z.string()),
  namespace: z.string(),
  status: z.enum(['pending', 'provisioning', 'ready', 'failed']),
  created_at: z.string(),
  viewers: z.array(ViewerSchema).default([]),
  project: z.string().default('default'),
})

export const CreateWorkspaceSchema = z.object({
  name: z
    .string()
    .min(1, 'Name is required')
    .max(63, 'Name must be at most 63 characters')
    .regex(/^[a-z0-9-]+$/, 'Name must contain only lowercase letters, digits, and hyphens'),
  repos: z.array(z.string().url('Must be a valid URL')).min(1, 'At least one repository is required'),
  viewers: z.array(z.string()),
  project: z.string().optional(),
})

export type Viewer = z.infer<typeof ViewerSchema>
export type Workspace = z.infer<typeof WorkspaceSchema>
export type CreateWorkspace = z.infer<typeof CreateWorkspaceSchema>

export const WsMetricsSchema = z.object({
  pod_count: z.number(),
  cpu_requests_millicores: z.number(),
  cpu_limits_millicores: z.number(),
  memory_requests_mi: z.number(),
  memory_limits_mi: z.number(),
})

export type WsMetrics = z.infer<typeof WsMetricsSchema>

/** Fetches all workspaces from the API. */
export async function fetchWorkspaces(): Promise<Workspace[]> {
  const response = await fetch('/api/workspaces')
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const data = await response.json()
  return z.array(WorkspaceSchema).parse(data)
}

/** Creates a new workspace via the API. */
export async function createWorkspace(data: CreateWorkspace): Promise<Workspace> {
  const response = await fetch('/api/workspaces', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const result = await response.json()
  return WorkspaceSchema.parse(result)
}

/** Fetches a single workspace by name. */
export async function fetchWorkspace(name: string): Promise<Workspace> {
  const response = await fetch(`/api/workspaces/${name}`)
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const data = await response.json()
  return WorkspaceSchema.parse(data)
}

/** Fetches resource metrics for a workspace. */
export async function fetchWorkspaceMetrics(name: string): Promise<WsMetrics> {
  const response = await fetch(`/api/workspaces/${name}/metrics`)
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const data = await response.json()
  return WsMetricsSchema.parse(data)
}

/** Deletes a workspace by name. */
export async function deleteWorkspace(name: string): Promise<void> {
  const response = await fetch(`/api/workspaces/${name}`, {
    method: 'DELETE',
  })
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
}
