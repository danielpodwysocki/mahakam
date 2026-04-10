// Proxy viewer requests to the per-environment viewer service.
// Path format: /projects/viewers/{env}/{viewerName}/...
// Service name: viewer-{env}-{viewerName}.env-{env}:80
// Only used when the Nuxt server is accessed directly (e.g. :3001 in dev).
// Production traffic goes through Envoy Gateway which routes directly to the service.
export default defineEventHandler(async (event) => {
  const env = getRouterParam(event, 'env')
  const rest = getRouterParam(event, 'path') ?? ''
  const viewerName = rest.split('/')[0]
  if (!viewerName) {
    setResponseStatus(event, 404)
    return { error: 'viewer name missing from path' }
  }
  const target = `http://viewer-${env}-${viewerName}.env-${env}:80/projects/viewers/${env}/${rest}`
  return proxyRequest(event, target)
})
