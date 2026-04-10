// Viewers are now accessed at /projects/viewers/{ws}/{viewerName}/.
// A bare /projects/viewers/{ws}/ path is not valid.
export default defineEventHandler((event) => {
  setResponseStatus(event, 404)
  return { error: 'specify a viewer name in the path' }
})
