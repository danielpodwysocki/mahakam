import { z } from 'zod'

export const EnvironmentSchema = z.object({
  id: z.string(),
  name: z.string(),
  repos: z.array(z.string()),
  namespace: z.string(),
  status: z.enum(['pending', 'provisioning', 'ready', 'failed']),
  created_at: z.string(),
})

export const CreateEnvironmentSchema = z.object({
  name: z
    .string()
    .min(1, 'Name is required')
    .max(63, 'Name must be at most 63 characters')
    .regex(/^[a-z0-9-]+$/, 'Name must contain only lowercase letters, digits, and hyphens'),
  repos: z.array(z.string().url('Must be a valid URL')).min(1, 'At least one repository is required'),
})

export type Environment = z.infer<typeof EnvironmentSchema>
export type CreateEnvironment = z.infer<typeof CreateEnvironmentSchema>

/** Fetches all environments from the API. */
export async function fetchEnvironments(): Promise<Environment[]> {
  const response = await fetch('/api/environments')
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const data = await response.json()
  return z.array(EnvironmentSchema).parse(data)
}

/** Creates a new environment via the API. */
export async function createEnvironment(data: CreateEnvironment): Promise<Environment> {
  const response = await fetch('/api/environments', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const result = await response.json()
  return EnvironmentSchema.parse(result)
}

/** Deletes an environment by name. */
export async function deleteEnvironment(name: string): Promise<void> {
  const response = await fetch(`/api/environments/${name}`, {
    method: 'DELETE',
  })
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
}
