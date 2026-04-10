export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig()
  const name = getRouterParam(event, 'name')
  try {
    return await $fetch(`${config.apiBaseUrl}/api/v1/workspaces/${name}`)
  } catch (error: any) {
    throw createError({ statusCode: error.statusCode ?? 500, statusMessage: error.message })
  }
})
