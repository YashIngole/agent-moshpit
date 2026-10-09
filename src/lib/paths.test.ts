import { describe, expect, it } from 'vitest'
import { findPaths } from './paths'

const paths = (text: string) => findPaths(text).map(p => [p.path, p.line])

describe('file paths in a terminal', () => {
  it('finds paths with folders, lines and drives', () => {
    expect(paths('● Update(src/cart/total.ts)')).toEqual([['src/cart/total.ts', null]])
    expect(paths('error at src/lib/office.svelte.ts:120:7 here')).toEqual([['src/lib/office.svelte.ts', 120]])
    expect(paths('see C:\\Users\\Yash\\Documents\\agent-moshpit\\README.md')).toEqual([['C:\\Users\\Yash\\Documents\\agent-moshpit\\README.md', null]])
    expect(paths('./tools/e2e/lib.mjs and ../up.rs')).toEqual([['./tools/e2e/lib.mjs', null], ['../up.rs', null]])
  })

  it('takes a bare name only when its extension is code', () => {
    expect(paths('edited total.ts and README.md')).toEqual([['total.ts', null], ['README.md', null]])
    expect(paths('built with Node.js, e.g. version 2.1.290')).toEqual([])
    expect(paths('notes.unknown:12')).toEqual([['notes.unknown', 12]])
  })

  it('leaves web addresses to the web links', () => {
    expect(paths('open https://example.com/docs/page.html now')).toEqual([])
    expect(paths('http://localhost:5173/src/main.ts then src/main.ts')).toEqual([['src/main.ts', null]])
  })

  it('says where each one is in the line', () => {
    const [found] = findPaths('  at src/a.ts:3')
    expect([found!.start, found!.length]).toEqual([5, 10])
  })
})
