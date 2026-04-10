// Viewers are now accessed at /projects/viewers/{env}/{viewerName}/.
// A bare /projects/viewers/{env}/ path is not valid.
export default defineEventHandler((event) => {
  setResponseStatus(event, 404)
  return { error: 'specify a viewer name in the path' }
})
