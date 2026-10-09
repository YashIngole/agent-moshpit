// How a person at a desk looks: clothes, hair, what they wear on their head and
// what is on their desk, picked from one stable number so the same agent always
// looks the same.
//
// Clothes are dark and muted on purpose: amber, green and red are status, and so
// is the light of a screen.

export type Top = 'hoodie' | 'tee' | 'jacket'
export type Hair = 'buzz' | 'swept' | 'messy' | 'long' | 'curly' | 'bun' | 'bald'
export type Headwear = 'none' | 'beanie' | 'cap' | 'headphones' | 'hood'
export type Glasses = 'none' | 'square' | 'round'
export type Beard = 'none' | 'stubble' | 'full'
export type Item = 'can' | 'mug' | 'lamp' | 'plant' | 'cans'

export interface Look {
  skin: string
  /** Lines of the face: darker on darker skin, so they still read. */
  ink: string
  hair: string
  hairStyle: Hair
  top: Top
  /** The colour of what they wear, and of what they wear on their head. */
  cloth: string
  hat: string
  headwear: Headwear
  glasses: Glasses
  beard: Beard
  item: Item
  /** A second, smaller screen beside the first. */
  dual: boolean
}

export const SKIN = ['#f3cfae', '#e6b48c', '#d09a6c', '#a8734e', '#80523a', '#5f3b28']
export const HAIR = ['#17171c', '#2e221b', '#5a3c28', '#7d4a2b', '#a89270', '#8f9099', '#d9d4ca', '#4f6fd6', '#8a62e0']
export const CLOTH = ['#16181d', '#272b33', '#3a4049', '#1d2a45', '#2f4b72', '#3f325f', '#dcd7cb', '#4b6386', '#5b5f6a']
const HATS = ['#121418', '#2b2f37', '#3f325f', '#1d2a45', '#cfc9bc', '#4b5262']

const TOPS: Top[] = ['hoodie', 'hoodie', 'tee', 'hoodie', 'jacket', 'tee']
const HAIRS: Hair[] = ['buzz', 'swept', 'messy', 'long', 'curly', 'bun', 'bald']
const HEADWEAR: Headwear[] = ['none', 'none', 'headphones', 'beanie', 'headphones', 'cap', 'none']
const ITEMS: Item[] = ['can', 'mug', 'lamp', 'plant', 'cans', 'mug']

function picker(seed: number) {
  let state = seed >>> 0 || 1
  return (count: number) => {
    state ^= state << 13
    state >>>= 0
    state ^= state >>> 17
    state ^= state << 5
    state >>>= 0
    return state % count
  }
}

export function lookFor(seed: number): Look {
  const pick = picker(seed)
  const tone = pick(SKIN.length)
  const hairStyle = HAIRS[pick(HAIRS.length)]!
  const headwear = HEADWEAR[pick(HEADWEAR.length)]!
  return {
    skin: SKIN[tone]!,
    ink: tone >= 4 ? '#140c08' : '#1d1b22',
    hair: HAIR[pick(HAIR.length)]!,
    hairStyle,
    top: TOPS[pick(TOPS.length)]!,
    cloth: CLOTH[pick(CLOTH.length)]!,
    hat: HATS[pick(HATS.length)]!,
    headwear,
    glasses: pick(3) === 0 ? (pick(2) ? 'square' : 'round') : 'none',
    beard: pick(5) === 0 ? 'full' : pick(4) === 0 ? 'stubble' : 'none',
    item: ITEMS[pick(ITEMS.length)]!,
    dual: pick(3) === 0
  }
}

/** A colour a step darker, for the folds of cloth. */
export function shade(hex: string, by = 0.72): string {
  const n = parseInt(hex.slice(1), 16)
  const c = (shift: number) => Math.round(((n >> shift) & 255) * by)
  return `#${[c(16), c(8), c(0)].map(v => v.toString(16).padStart(2, '0')).join('')}`
}

/** Pale cloth needs dark strings and folds; dark cloth needs light ones. */
export function pale(hex: string): boolean {
  const n = parseInt(hex.slice(1), 16)
  return ((n >> 16) & 255) * 0.299 + ((n >> 8) & 255) * 0.587 + (n & 255) * 0.114 > 150
}
