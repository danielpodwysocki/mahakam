const apiBase = process.env.API_BASE_URL || 'http://localhost:3000'

export default defineNuxtConfig({
  devtools: { enabled: false },
  routeRules: {
    '/backend/**': {
      proxy: `${apiBase}/**`,
    },
  },
})
