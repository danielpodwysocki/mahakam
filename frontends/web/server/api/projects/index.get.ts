export default defineEventHandler(async () => {
  const config = useRuntimeConfig()
  try {
    return await $fetch(`${config.apiBaseUrl}/api/v1/projects`)
  } catch (error: any) {
    throw createError({ statusCode: error.statusCode ?? 500, statusMessage: error.message })
  }
})
