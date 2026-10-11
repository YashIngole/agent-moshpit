import { describe as group, expect, it } from 'vitest'
import type { Agent } from './types'
import { STATUS_WORD, describe, doing, mixed, place, resumeAction, resumeHint, shorten, titleFrom, uniqueTitle } from './words'

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

  it('says what the program or the agent said, when it said something', () => {
    expect(doing(agent({ phase: 'needs_you', activity: 'Approval requested: rm -rf dist' }))).toBe('Approval requested: rm -rf dist')
    expect(doing(agent({ phase: 'failed', activity: 'Codex was not found on this computer.' }))).toBe('Codex was not found on this computer.')
    expect(doing(agent({ phase: 'working', activity: 'Reading checkout tests' }))).toBe('Reading checkout tests')
    expect(doing(agent({ phase: 'done', activity: 'Typed the orders API' }))).toBe('Typed the orders API')
  })

  it('says nothing that the status word already says', () => {
    expect(doing(agent())).toBe('')
    expect(doing(agent({ phase: 'needs_you' }))).toBe('')
    expect(doing(agent({ phase: 'needs_you', activity: 'Waiting for your answer in the terminal' }))).toBe('')
    expect(doing(agent({ phase: 'needs_you', activity: 'Waiting for your input' }))).toBe('')
    expect(doing(agent({ phase: 'done' }))).toBe('')
    expect(doing(agent({ phase: 'idle' }))).toBe('')
  })

  it('says opening someone away shows where they left off, and promises nothing more', () => {
    // Opening a desk no longer starts its program: Carry on and Start again are buttons in the pane.
    for (const resumable of [true, false]) {
      const away = agent({ phase: 'asleep', running: false, resumable })
      expect(doing(away)).toBe('')
      expect(describe(away)).not.toMatch(/carry on|start again/i)
    }
  })

  it('reads a whole desk aloud, program and place included, and never says the status twice', () => {
    expect(describe(agent({ phase: 'done' }))).toBe('Fix the checkout total, Claude Code, in shop. Done.')
    expect(describe(agent({ repo: '', phase: 'idle' }))).toBe('Fix the checkout total, Claude Code. Idle.')
    expect(describe(agent({ activity: 'Reading checkout tests' }))).toBe('Fix the checkout total, Claude Code, in shop. Working. Reading checkout tests')
  })

  it('knows when an office runs more than one program', () => {
    expect(mixed([agent(), agent({ id: 'b' })])).toBe(false)
    expect(mixed([agent(), agent({ id: 'b', harness: 'codex' })])).toBe(true)
    expect(mixed([])).toBe(false)
  })

  it('names the project and the branch when there is one', () => {
    expect(place(agent())).toBe('shop · fix/checkout-total')
    expect(place(agent({ branch: '' }))).toBe('shop')
    expect(place(agent({ repo: '', branch: '' }))).toBe('')
  })

  it('does not promise completion or desk identity when they are unknown', () => {
    expect(describe(agent({ phase: 'quiet', activity: 'No recent output. It may still be working.' }))).toContain('Quiet. No recent output. It may still be working.')
    expect(resumeAction(agent({ resume_scope: 'desk' }))).toBe('Carry on')
    expect(resumeAction(agent({ resume_scope: 'folder' }))).toBe('Continue in folder')
    expect(resumeAction(agent({ resume_scope: 'latest' }))).toBe('Continue latest')
    expect(resumeHint(agent({ resume_scope: 'latest' }))).toContain('may belong to another desk')
    expect(resumeAction(agent({ resumable: false, resume_scope: 'none' }))).toBe('Start again')
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
