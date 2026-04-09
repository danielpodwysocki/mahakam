export default defineNuxtConfig({
  devtools: { enabled: false },
  runtimeConfig: {
    apiBaseUrl: 'http://localhost:3000',
  },
  nitro: {
    routeRules: {
      // Allow the viewer iframe to render inside this app.
      // DENY would block the terminal panel from loading.
      '/**': { headers: { 'X-Frame-Options': 'SAMEORIGIN' } },
    },
  },
})
