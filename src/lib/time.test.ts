import { describe, expect, it } from 'vitest'
import { countdown, elapsed, took } from './time'

describe('took', () => {
  it('gives a short wait its tenths and a long one whole units', () => {
    expect(took(0.42)).toBe('0.4 sec')
    expect(took(3.25)).toBe('3.3 sec')
    expect(took(8)).toBe('8 sec')
    expect(took(9.96)).toBe('10 sec')
    expect(took(12.4)).toBe('12 sec')
    expect(took(59.6)).toBe('1 min')
    expect(took(102)).toBe('1 min 42 sec')
    expect(took(3600)).toBe('1 hr')
    expect(took(7_500)).toBe('2 hr 5 min')
  })

  it('says nothing about a time that makes no sense', () => {
    expect(took(-1)).toBe('')
    expect(took(Number.NaN)).toBe('')
  })
})

describe('countdown', () => {
  it('shows minutes and two-digit seconds', () => {
    expect(countdown(252_000)).toBe('4:12')
    expect(countdown(60_000)).toBe('1:00')
    expect(countdown(9_000)).toBe('0:09')
  })

  it('rounds up, so the last second is not shown as zero', () => {
    expect(countdown(1)).toBe('0:01')
    expect(countdown(59_001)).toBe('1:00')
  })

  it('never goes below zero', () => {
    expect(countdown(0)).toBe('0:00')
    expect(countdown(-5_000)).toBe('0:00')
  })
})

describe('elapsed', () => {
  it('moves through seconds, minutes, hours and days', () => {
    expect(elapsed(0)).toBe('just now')
    expect(elapsed(9_999)).toBe('just now')
    expect(elapsed(10_000)).toBe('10 sec')
    expect(elapsed(59_999)).toBe('59 sec')
    expect(elapsed(60_000)).toBe('1 min')
    expect(elapsed(59 * 60_000)).toBe('59 min')
    expect(elapsed(60 * 60_000)).toBe('1 hr')
    expect(elapsed(47 * 3_600_000)).toBe('47 hr')
    expect(elapsed(48 * 3_600_000)).toBe('2 days')
  })

  it('treats a clock that ran backwards as just now', () => {
    expect(elapsed(-30_000)).toBe('just now')
  })
})
