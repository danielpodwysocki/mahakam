export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig()
  const name = getRouterParam(event, 'name')
  const viewer = getRouterParam(event, 'viewer')
  try {
    await $fetch(
      `${config.apiBaseUrl}/api/v1/workspaces/${name}/viewers/${viewer}/restart`,
      { method: 'POST' },
    )
  } catch (error: any) {
    throw createError({ statusCode: error.statusCode ?? 500, statusMessage: error.message })
  }
  setResponseStatus(event, 204)
})
