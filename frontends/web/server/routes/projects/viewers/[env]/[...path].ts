// Proxy viewer requests to the per-workspace viewer service.
// Path format: /projects/viewers/{ws}/{viewerName}/...
// Service name: viewer-{ws}-{viewerName}.ws-{ws}:80
// Only used when the Nuxt server is accessed directly (e.g. :3001 in dev).
// Production traffic goes through Envoy Gateway which routes directly to the service.
export default defineEventHandler(async (event) => {
  const ws = getRouterParam(event, 'env')
  const rest = getRouterParam(event, 'path') ?? ''
  const viewerName = rest.split('/')[0]
  if (!viewerName) {
    setResponseStatus(event, 404)
    return { error: 'viewer name missing from path' }
  }
  const target = `http://viewer-${ws}-${viewerName}.ws-${ws}:80/projects/viewers/${ws}/${rest}`
  return proxyRequest(event, target)
})
