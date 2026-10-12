import { describe, expect, it } from 'vitest'
import { deliveryWords, linkWords, taskWords, type Delivery, type Link, type MailDesk } from './messages'

const delivery = (over: Partial<Delivery>): Delivery => ({ state: 'saved', revision: 1, notified_ms: null, notified_via: '', fetched_ms: null, acknowledged_ms: null, ...over })
const link = (over: Partial<Link>): Link => ({ granted: true, live: true, client: 'claude-code 2.1.296', push: 'poll', channel: 'off', hooks: 'off', hook_seen_ms: null, wake: false, note: '', ...over })
const desk = (over: Partial<MailDesk>, of: Partial<Link> = {}): MailDesk => ({ id: 'a', title: 'A', harness: 'claude', harness_name: 'Claude Code', running: true, waiting: 0, link: link(of), ...over })

describe('how far a message got is said in words that claim only what is known', () => {
  it('says each step as its own thing', () => {
    expect(deliveryWords(delivery({}))).toBe('Saved. Not told yet.')
    expect(deliveryWords(delivery({ state: 'notified', notified_via: 'channel', notified_ms: 1 }))).toMatch(/^Pushed .*\. Not known to have arrived\.$/)
    expect(deliveryWords(delivery({ state: 'notified', notified_via: 'stop', notified_ms: 1 }))).toMatch(/^Said as its turn ended .*\. Not read yet\.$/)
    expect(deliveryWords(delivery({ state: 'notified', notified_via: 'wake', notified_ms: 1 }))).toMatch(/^It was woken for this/)
    expect(deliveryWords(delivery({ state: 'notified', notified_via: 'inbox', notified_ms: 1 }))).toMatch(/Not read in full\.$/)
    expect(deliveryWords(delivery({ state: 'fetched', fetched_ms: 1 }))).toMatch(/^Read in full .*\. Not acknowledged\.$/)
    expect(deliveryWords(delivery({ state: 'acknowledged', acknowledged_ms: 1 }))).toMatch(/^Acknowledged /)
    expect(deliveryWords(null)).toBe('Handed over as its session started.')
  })
  it('never says that a pushed or mentioned message was read', () => {
    for (const via of ['channel', 'hook', 'stop', 'wake', 'hint', 'inbox']) {
      const words = deliveryWords(delivery({ state: 'notified', notified_via: via, notified_ms: 1 }))
      expect(words).toMatch(/Not (read|known)/)
      expect(words).not.toMatch(/^Read|Acknowledged/)
    }
  })
})

describe('how a session is reached', () => {
  it('tells a stopped one, a starting one and a connected one apart', () => {
    expect(linkWords(desk({ running: false }))).toMatch(/^Not running\. What is sent is kept/)
    expect(linkWords(desk({}, { live: false }))).toBe('Started. Its program has not connected to the office yet.')
    expect(linkWords(desk({}))).toMatch(/next call to the office, or when it checks its inbox/)
  })
  it('says hooks were given until one has run, and pushes are on until one is known to arrive', () => {
    expect(linkWords(desk({}, { hooks: 'configured' }))).toBe('Connected: given hooks, which have not run yet.')
    expect(linkWords(desk({}, { hooks: 'seen', wake: true }))).toBe('Connected: told at a step of its work, and woken when idle.')
    expect(linkWords(desk({}, { hooks: 'seen' }))).toBe('Connected: told at a step of its work or its next prompt.')
    expect(linkWords(desk({}, { channel: 'requested' }))).toBe('Pushes are on; none is known to have arrived yet.')
    expect(linkWords(desk({}, { channel: 'confirmed', hooks: 'seen', wake: true }))).toBe('Pushes reach it. Also told at a step of its work, and woken when idle.')
  })
  it('gives a task its own words', () => {
    expect([taskWords('queued'), taskWords('in_progress'), taskWords('completed'), taskWords(undefined)]).toEqual(['not taken up yet', 'taken up', 'done', ''])
  })
})
