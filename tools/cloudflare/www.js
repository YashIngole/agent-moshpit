// www.agentmoshpit.com is only another way to type the address.
export default {
  fetch(request) {
    const url = new URL(request.url)
    url.hostname = 'agentmoshpit.com'
    return Response.redirect(url.toString(), 301)
  }
}
