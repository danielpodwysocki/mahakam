export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig()
  const name = getRouterParam(event, 'name')
  try {
    await $fetch(`${config.apiBaseUrl}/api/v1/workspaces/${name}`, { method: 'DELETE' })
  } catch (error: any) {
    throw createError({ statusCode: error.statusCode ?? 500, statusMessage: error.message })
  }
  setResponseStatus(event, 204)
})
