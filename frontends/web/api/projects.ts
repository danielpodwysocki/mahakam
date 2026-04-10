import { z } from 'zod'

export const ProjectSchema = z.object({
  name: z.string(),
  workspace_count: z.number(),
})

export type Project = z.infer<typeof ProjectSchema>

/** Fetches all projects from the API. */
export async function fetchProjects(): Promise<Project[]> {
  const response = await fetch('/api/projects')
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  const data = await response.json()
  return z.array(ProjectSchema).parse(data)
}
