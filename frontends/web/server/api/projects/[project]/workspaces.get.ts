export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig()
  const project = getRouterParam(event, 'project')
  try {
    return await $fetch(`${config.apiBaseUrl}/api/v1/projects/${project}/workspaces`)
  } catch (error: any) {
    throw createError({ statusCode: error.statusCode ?? 500, statusMessage: error.message })
  }
})
