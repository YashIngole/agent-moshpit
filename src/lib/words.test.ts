import { describe as group, expect, it } from 'vitest'
import type { Agent } from './types'
import { STATUS_WORD, describe, doing, place, shorten, titleFrom, uniqueTitle } from './words'

function agent(over: Partial<Agent> = {}): Agent {
  return {
    id: 'a',
    title: 'Fix the checkout total',
    phase: 'working',
    activity: '',
    harness: 'claude',
    harness_name: 'Claude Code',
    harness_tag: 'Claude',
    project: 'C:/code/shop',
    repo: 'shop',
    branch: 'fix/checkout-total',
    cwd: 'C:/code/shop',
    since_ms: 0,
    look: 1,
    running: true,
    resumable: true,
    resume_note: '',
    unread: false,
    ...over
  }
}

group('what a desk says', () => {
  it('has a word for every status', () => {
    for (const word of Object.values(STATUS_WORD)) expect(word.length).toBeGreaterThan(0)
  })

  it('says what the program said, when it said something', () => {
    expect(doing(agent({ phase: 'needs_you', activity: 'Asked you a question' }))).toBe('Asked you a question')
    expect(doing(agent({ phase: 'needs_you' }))).toBe('Waiting for you in their terminal')
    expect(doing(agent({ phase: 'failed', activity: 'Codex was not found on this computer.' }))).toBe('Codex was not found on this computer.')
    expect(doing(agent())).toBe('Working')
  })

  it('says opening someone away shows where they left off, and promises nothing more', () => {
    // Opening a desk no longer starts its program: Carry on and Start again are buttons in the pane.
    for (const resumable of [true, false]) {
      const away = agent({ phase: 'asleep', running: false, resumable })
      expect(doing(away)).toBe('Not running. Open to see where they left off.')
      expect(describe(away)).not.toMatch(/carry on|start again/i)
    }
  })

  it('reads a whole desk aloud, program and place included', () => {
    expect(describe(agent({ phase: 'done' }))).toBe('Fix the checkout total, Claude Code, in shop. Done. Finished. Open their terminal to see what they did.')
    expect(describe(agent({ repo: '', phase: 'idle' }))).toBe('Fix the checkout total, Claude Code. Idle. Waiting for something to do')
  })

  it('names the project and the branch when there is one', () => {
    expect(place(agent())).toBe('shop · fix/checkout-total')
    expect(place(agent({ branch: '' }))).toBe('shop')
    expect(place(agent({ repo: '', branch: '' }))).toBe('')
  })
})

group('names made for a desk', () => {
  it('come from the start of the task, as the core makes them', () => {
    // The same cases as `titles_come_from_the_start_of_the_task` in office.rs.
    expect(titleFrom('Fix the checkout total when a coupon is applied twice.')).toBe('Fix the checkout total')
    expect(titleFrom('Make the checkout total right when a coupon is applied')).toBe('Make the checkout total right')
    expect(titleFrom('Refactor auth.\nThen add tests.')).toBe('Refactor auth')
    expect(titleFrom('Fix the')).toBe('Fix the')
    expect(shorten('a'.repeat(50), 40)).toBe(`${'a'.repeat(39)}…`)
  })

  it('are numbered when another desk has the name', () => {
    expect(uniqueTitle('Fake in w3', ['Fake in w3'])).toBe('Fake in w3 2')
    expect(uniqueTitle('Fix the total', ['Fix the total', 'Fix the total 2'])).toBe('Fix the total 3')
    expect(uniqueTitle('Something new', ['Fix the total'])).toBe('Something new')
  })
})
