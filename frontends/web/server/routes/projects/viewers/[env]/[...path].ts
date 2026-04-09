// Proxy viewer terminal requests to the per-environment ttyd service.
// This lets the web app be accessed directly (e.g. via Skaffold port-forward
// on :3001) without the Envoy Gateway in the path.
export default defineEventHandler(async (event) => {
  const env = getRouterParam(event, 'env')
  const rest = getRouterParam(event, 'path') ?? ''
  const target = `http://viewer-${env}.env-${env}:7681/projects/viewers/${env}/${rest}`
  return proxyRequest(event, target)
})
