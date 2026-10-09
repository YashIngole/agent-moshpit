import { describe, expect, it } from 'vitest'
import { CLOTH, lookFor, pale, shade } from './look'

/** Hue in degrees and saturation 0..1 of a #rrggbb colour. */
function hueOf(hex: string): { hue: number; saturation: number } {
  const n = parseInt(hex.slice(1), 16)
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map(c => c / 255) as [number, number, number]
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const spread = max - min
  if (spread === 0) return { hue: 0, saturation: 0 }
  const lightness = (max + min) / 2
  const saturation = spread / (1 - Math.abs(2 * lightness - 1))
  const raw = max === r ? ((g - b) / spread) % 6 : max === g ? (b - r) / spread + 2 : (r - g) / spread + 4
  return { hue: (raw * 60 + 360) % 360, saturation }
}

const everyone = Array.from({ length: 4000 }, (_, seed) => lookFor(seed * 7919 + 1))

describe('how a person looks', () => {
  it('is the same every time for the same agent', () => {
    expect(lookFor(0x51a7c3)).toEqual(lookFor(0x51a7c3))
  })

  it('varies between agents', () => {
    expect(new Set(everyone.map(p => p.skin)).size).toBe(6)
    expect(new Set(everyone.map(p => p.hairStyle)).size).toBe(7)
    expect(new Set(everyone.map(p => p.headwear)).size).toBe(4)
    expect(new Set(everyone.map(p => p.top)).size).toBe(3)
    expect(new Set(everyone.map(p => p.item)).size).toBe(5)
    expect(everyone.some(p => p.glasses !== 'none') && everyone.some(p => p.beard !== 'none') && everyone.some(p => p.dual)).toBe(true)
  })

  it('copes with a seed of zero', () => {
    expect(lookFor(0).skin).toMatch(/^#[0-9a-f]{6}$/)
  })

  it('never dresses anyone in a status colour', () => {
    // Amber means "needs you", green means done and red means trouble.
    for (const cloth of CLOTH) {
      const { hue, saturation } = hueOf(cloth)
      if (saturation < 0.35) continue
      const amber = hue >= 30 && hue <= 75
      const green = hue >= 80 && hue <= 165
      const red = hue >= 345 || hue <= 20
      expect({ cloth, amber, green, red }).toEqual({ cloth, amber: false, green: false, red: false })
    }
  })

  it('draws faces in a darker line on darker skin', () => {
    expect(everyone.find(p => p.skin === '#5f3b28')?.ink).toBe('#140c08')
    expect(everyone.find(p => p.skin === '#f3cfae')?.ink).toBe('#1d1b22')
  })

  it('folds cloth a shade darker, and knows pale cloth from dark', () => {
    expect(shade('#ffffff', 0.5)).toBe('#808080')
    expect(pale('#dcd7cb')).toBe(true)
    expect(pale('#16181d')).toBe(false)
  })
})
