// Crawl the served site before and after deployment:
//   node tools/check-site.mjs http://127.0.0.1:4177
//   node tools/check-site.mjs https://agentmoshpit.com
// Uses the Edge or Chrome already installed. No signed-in browser is needed.
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { chromium } from 'playwright-core'

const origin = new URL(process.argv[2] || 'https://agentmoshpit.com').origin
const canonicalOrigin = 'https://agentmoshpit.com'
const version = JSON.parse(await readFile(new URL('../package.json', import.meta.url))).version
const response = await fetch(`${origin}/sitemap.xml`)
assert.equal(response.status, 200, 'sitemap status')
const sitemap = await response.text()
const pages = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map(match => new URL(match[1]))
assert.ok(pages.length >= 6, 'home, guide index and four guides in sitemap')
assert.equal(new Set(pages.map(url => url.href)).size, pages.length, 'unique sitemap URLs')
for (const url of pages) assert.equal(url.origin, canonicalOrigin, 'sitemap uses canonical host')
const robots = await fetch(`${origin}/robots.txt`)
assert.equal(robots.status, 200, 'robots status')
assert.match(await robots.text(), /Sitemap: https:\/\/agentmoshpit\.com\/sitemap\.xml/)

let browser
for (const channel of ['msedge', 'chrome', undefined]) {
  try { browser = await chromium.launch({ channel, headless: true }); break } catch {}
}
assert.ok(browser, 'an installed browser is required')
const checkedLinks = new Set()
const checkedAssets = new Set()
const titles = new Set()
const descriptions = new Set()
try {
  // Searchable text and navigation must work with scripting disabled.
  const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 1280, height: 900 } })
  const page = await context.newPage()
  for (const url of pages) {
    const target = new URL(url.pathname, origin).href
    const navigation = await page.goto(target, { waitUntil: 'load' })
    assert.equal(navigation.status(), 200, `${url.pathname}: status`)
    assert.doesNotMatch(navigation.headers()['x-robots-tag'] || '', /noindex/, `${url.pathname}: indexable`)
    const data = await page.evaluate(() => ({
      title: document.title,
      description: document.querySelector('meta[name="description"]')?.content,
      robots: document.querySelector('meta[name="robots"]')?.content || '',
      canonical: document.querySelector('link[rel="canonical"]')?.href,
      og: Object.fromEntries([...document.querySelectorAll('meta[property^="og:"]')].map(meta => [meta.getAttribute('property'), meta.content])),
      twitter: document.querySelector('meta[name="twitter:card"]')?.content,
      headings: [...document.querySelectorAll('h1,h2,h3,h4,h5,h6')].map(h => Number(h.tagName[1])),
      text: document.querySelector('main')?.textContent.replace(/\s+/g, ' ').trim(),
      schemas: [...document.querySelectorAll('script[type="application/ld+json"]')].flatMap(script => {
        const data = JSON.parse(script.textContent)
        return data['@graph'] || [data]
      }),
      questions: [...document.querySelectorAll('.asked dt')].map(dt => dt.textContent.replace(/\s+/g, ' ').trim()),
      links: [...document.querySelectorAll('a[href]')].map(a => a.href),
      assets: [...document.querySelectorAll('img[src],link[rel="stylesheet"],link[rel="preload"],link[rel="icon"]')].map(el => el.src || el.href),
      images: [...document.images].map(img => ({ alt: img.getAttribute('alt'), width: img.getAttribute('width'), height: img.getAttribute('height') }))
    }))
    assert.ok(data.title && !titles.has(data.title), `${url.pathname}: unique title`)
    titles.add(data.title)
    assert.ok(data.description && !descriptions.has(data.description), `${url.pathname}: unique description`)
    descriptions.add(data.description)
    assert.equal(data.canonical, url.href, `${url.pathname}: canonical`)
    assert.equal(data.og['og:url'], url.href, `${url.pathname}: social URL`)
    for (const key of ['og:title', 'og:description', 'og:image', 'og:image:alt']) assert.ok(data.og[key], `${url.pathname}: ${key}`)
    assert.equal(data.twitter, 'summary_large_image')
    assert.doesNotMatch(data.robots, /noindex/)
    assert.equal(data.headings.filter(level => level === 1).length, 1, `${url.pathname}: one h1`)
    data.headings.forEach((level, i) => assert.ok(i === 0 || level <= data.headings[i - 1] + 1, `${url.pathname}: heading order`))
    assert.ok(data.text.length > 600, `${url.pathname}: readable without JavaScript`)
    assert.ok(data.schemas.length, `${url.pathname}: structured data`)
    const application = data.schemas.find(schema => schema['@type'] === 'SoftwareApplication')
    if (url.pathname === '/') {
      assert.ok(application)
      assert.equal(application.softwareVersion, version)
      assert.equal(Number(application.offers.price), 0)
      for (const key of ['operatingSystem', 'applicationCategory', 'downloadUrl']) assert.ok(application[key])
      assert.ok(data.schemas.some(schema => schema['@type'] === 'WebSite'))
    }
    const faq = data.schemas.find(schema => schema['@type'] === 'FAQPage')
    if (faq) for (const question of faq.mainEntity) {
      assert.ok(data.questions.includes(question.name), `${url.pathname}: FAQ question is visible`)
      assert.equal(question.acceptedAnswer['@type'], 'Answer')
      assert.ok(question.acceptedAnswer.text)
    }
    for (const image of data.images) {
      assert.notEqual(image.alt, null, `${url.pathname}: image alt attribute`)
      assert.ok(Number(image.width) > 0 && Number(image.height) > 0, `${url.pathname}: image reserves its size`)
    }
    for (const href of data.links) {
      const link = new URL(href)
      if (link.origin !== origin && link.origin !== canonicalOrigin) continue
      link.host = new URL(origin).host
      link.protocol = new URL(origin).protocol
      if (checkedLinks.has(link.href)) continue
      checkedLinks.add(link.href)
      const result = await context.request.get(link.href)
      assert.equal(result.status(), 200, `internal link ${link.href}`)
      if (link.hash) {
        const check = await context.newPage()
        await check.goto(link.href)
        assert.ok(await check.evaluate(hash => Boolean(document.getElementById(decodeURIComponent(hash.slice(1)))), link.hash), `fragment ${link.href}`)
        await check.close()
      }
    }
    for (const asset of [...data.assets, new URL(data.og['og:image']).pathname]) {
      const target = new URL(asset, origin)
      if (checkedAssets.has(target.href)) continue
      checkedAssets.add(target.href)
      const result = await context.request.get(target.href)
      assert.equal(result.status(), 200, `asset ${target.href}`)
      if (/\.webp$/.test(target.pathname)) assert.ok((await result.body()).length < 600_000, `image weight ${target.pathname}`)
    }
    console.log(`PASS ${url.pathname} (metadata, schema, headings, links, images, no JavaScript)`)
  }
  for (const path of ['/demo/', '/demo/index.html', '/install.sh', '/install.ps1']) {
    const result = await context.request.get(`${origin}${path}`)
    assert.equal(result.status(), 200, path)
    assert.match(result.headers()['x-robots-tag'] || '', /noindex/, `${path}: excluded from search`)
    if (path.startsWith('/install.')) assert.match(result.headers()['content-type'], /text\/plain/)
  }
  const missing = await context.request.get(`${origin}/__seo-check-missing-page__`)
  assert.equal(missing.status(), 404, 'missing URL returns a real 404')
  assert.match(await missing.text(), /Nobody sits at this desk/, 'custom 404 page')
  await context.close()
  const scripted = await browser.newPage()
  const errors = []
  scripted.on('pageerror', error => errors.push(error.message))
  await scripted.goto(origin)
  await scripted.waitForTimeout(1500)
  assert.equal(await scripted.locator('script[type="application/ld+json"]').count(), 1)
  assert.ok(await scripted.locator('button.copy:not([hidden])').count() >= 2, 'install command copy buttons work')
  assert.deepEqual(errors, [], 'no JavaScript errors')
  console.log(`PASS indexing headers, install scripts, 404 and JavaScript. ${checkedLinks.size} internal links; ${checkedAssets.size} assets.`)
} finally {
  await browser.close()
}
