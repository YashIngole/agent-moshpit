import { describe, expect, it } from 'vitest'
import { webAddress } from './links'

describe('addresses in a terminal', () => {
  it('opens https pages, and plain http only on this computer', () => {
    expect(webAddress('https://example.com/a?b=c#d')).toEqual({ href: 'https://example.com/a?b=c#d', host: 'example.com' })
    expect(webAddress('http://127.0.0.1:8080/health')?.host).toBe('127.0.0.1:8080')
    expect(webAddress('http://[::1]:3000/')?.host).toBe('[::1]:3000')
    expect(webAddress('http://localhost')?.host).toBe('localhost')
  })

  it('shows the real host of an address written to look like another', () => {
    // The first letter is a Cyrillic а, which looks the same as the Latin one.
    expect(webAddress('https://аpple.example/')?.host).toBe('xn--pple-43d.example')
  })

  it('refuses everything else', () => {
    for (const bad of ['http://example.com', 'https://user:pw@example.com/', 'https://a@b/', 'https://', 'ftp://example.com', 'https://exa mple.com', 'https://example.com\\@evil.example', 'not an address', '']) {
      expect(webAddress(bad)).toBeNull()
    }
  })
})
