export default defineEventHandler(async (event) => {
  const config = useRuntimeConfig()
  const body = await readBody(event)
  try {
    return await $fetch(`${config.apiBaseUrl}/api/v1/environments`, {
      method: 'POST',
      body: body as Record<string, unknown>,
    })
  } catch (error: any) {
    throw createError({ statusCode: error.statusCode ?? 500, statusMessage: error.message })
  }
})
