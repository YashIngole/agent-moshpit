// How the open terminals share the window: rows of panes, each with a share of
// the room. One pane fills it; two sit side by side; more make a grid. Any edge
// between two can be dragged.
//
// Everything here is a plain function of the layout it is given, so it can be
// tested without a window.

export interface Pane {
  /** The desk whose terminal this is. */
  id: string
  /** Its share of the row's width, against the other panes in the row. */
  size: number
}

export interface Row {
  /** Its share of the height, against the other rows. */
  size: number
  panes: Pane[]
}

export interface Layout {
  rows: Row[]
}

export const EMPTY: Layout = { rows: [] }

/** The least share a pane or row can be dragged down to, against 1 for an even share. */
const LEAST = 0.15

export function ids(layout: Layout): string[] {
  return layout.rows.flatMap(row => row.panes.map(pane => pane.id))
}

export function has(layout: Layout, id: string): boolean {
  return ids(layout).includes(id)
}

/**
 * The shape for this many panes: as close to square as it gets, wider than tall,
 * with any longer rows first. 1 → [1], 2 → [2], 3 → [2, 1], 5 → [3, 2], 7 → [3, 2, 2].
 */
export function shape(count: number): number[] {
  if (count <= 0) return []
  const columns = Math.ceil(Math.sqrt(count))
  const rows = Math.ceil(count / columns)
  const each = Math.floor(count / rows)
  const longer = count % rows
  return Array.from({ length: rows }, (_, i) => each + (i < longer ? 1 : 0))
}

/** These panes, in this order, laid out evenly. */
export function arrange(list: string[]): Layout {
  const unique = list.filter((id, i) => list.indexOf(id) === i)
  let next = 0
  return {
    rows: shape(unique.length).map(length => ({
      size: 1,
      panes: unique.slice(next, (next += length)).map(id => ({ id, size: 1 }))
    }))
  }
}

const mean = (values: number[]) => (values.length > 0 ? values.reduce((total, value) => total + value, 0) / values.length : 1)
/** Whether a row's panes all have the same share: nobody has dragged its edges. */
const evenRow = (panes: Pane[]) => panes.every(pane => Math.abs(pane.size - panes[0]!.size) < 1e-6)

/**
 * One more pane, beside `beside` (else beside the last), and nothing else moves.
 * It joins its neighbour's row when that row has room (a row may be as wide as
 * the grid is tall, so the room stays close to square); otherwise the shortest
 * row with room, or a new row at the bottom. A row nobody dragged stays even; in
 * one that was dragged, the neighbour gives the newcomer half its share.
 */
export function open(layout: Layout, id: string, beside?: string): Layout {
  if (has(layout, id)) return layout
  const list = ids(layout)
  if (list.length === 0) return { rows: [{ size: 1, panes: [{ id, size: 1 }] }] }
  const widest = Math.ceil(Math.sqrt(list.length + 1))
  const anchor = beside !== undefined && list.includes(beside) ? beside : list.at(-1)!
  const anchorRow = layout.rows.findIndex(row => row.panes.some(pane => pane.id === anchor))
  let target = anchorRow
  if (layout.rows[target]!.panes.length >= widest) {
    const roomy = layout.rows.map((row, at) => ({ at, length: row.panes.length })).filter(row => row.length < widest)
    target = roomy.length > 0 ? roomy.reduce((best, row) => (row.length < best.length ? row : best)).at : -1
  }
  if (target < 0) return { rows: [...layout.rows, { size: mean(layout.rows.map(row => row.size)), panes: [{ id, size: 1 }] }] }
  return {
    rows: layout.rows.map((row, at) => {
      if (at !== target) return row
      const near = at === anchorRow ? row.panes.findIndex(pane => pane.id === anchor) : row.panes.length - 1
      if (evenRow(row.panes)) {
        const panes = row.panes.map(pane => ({ ...pane, size: 1 }))
        panes.splice(near + 1, 0, { id, size: 1 })
        return { ...row, panes }
      }
      const half = row.panes[near]!.size / 2
      const panes = row.panes.map((pane, i) => (i === near ? { ...pane, size: half } : pane))
      panes.splice(near + 1, 0, { id, size: half })
      return { ...row, panes }
    })
  }
}

/** The same pane showing a different desk. Sizes stay as they were dragged. */
export function swap(layout: Layout, from: string, to: string): Layout {
  if (from === to || !has(layout, from)) return layout
  // Already open elsewhere: nothing is shown twice, so the pane that had it goes.
  const base = has(layout, to) ? close(layout, to) : layout
  return { rows: base.rows.map(row => ({ ...row, panes: row.panes.map(pane => (pane.id === from ? { ...pane, id: to } : pane)) })) }
}

/**
 * Without this pane, and nothing else moves: its share goes to the pane beside it
 * (a row nobody dragged stays even), and a row it leaves empty goes.
 */
export function close(layout: Layout, id: string): Layout {
  if (!has(layout, id)) return layout
  const rows = layout.rows
    .map(row => {
      const at = row.panes.findIndex(pane => pane.id === id)
      if (at < 0) return row
      const left = row.panes.filter(pane => pane.id !== id)
      if (left.length === 0 || evenRow(row.panes)) return { ...row, panes: left.map(pane => ({ ...pane, size: 1 })) }
      const heir = Math.max(0, at - 1)
      const share = row.panes[at]!.size
      return { ...row, panes: left.map((pane, i) => (i === heir ? { ...pane, size: pane.size + share } : pane)) }
    })
    .filter(row => row.panes.length > 0)
  return { rows }
}

/** Two open panes change places, each taking the other's place and size. */
export function trade(layout: Layout, a: string, b: string): Layout {
  if (a === b || !has(layout, a) || !has(layout, b)) return layout
  return {
    rows: layout.rows.map(row => ({ ...row, panes: row.panes.map(pane => (pane.id === a ? { ...pane, id: b } : pane.id === b ? { ...pane, id: a } : pane)) }))
  }
}

/**
 * Move the edge after pane `index` of row `row` to `at`: how far across the row
 * it should now stand, from 0 to 1. Only the two panes on either side change.
 */
export function moveColumnEdge(layout: Layout, row: number, index: number, at: number): Layout {
  const target = layout.rows[row]
  if (!target || index < 0 || index >= target.panes.length - 1) return layout
  const sizes = split(target.panes.map(pane => pane.size), index, at)
  return { rows: layout.rows.map((r, i) => (i === row ? { ...r, panes: r.panes.map((pane, j) => ({ ...pane, size: sizes[j]! })) } : r)) }
}

/** Move the edge under row `index` to `at`: how far down the room it should now stand. */
export function moveRowEdge(layout: Layout, index: number, at: number): Layout {
  if (index < 0 || index >= layout.rows.length - 1) return layout
  const sizes = split(layout.rows.map(row => row.size), index, at)
  return { rows: layout.rows.map((row, i) => ({ ...row, size: sizes[i]! })) }
}

/** Every pane and row an even share again. */
export function even(layout: Layout): Layout {
  return { rows: layout.rows.map(row => ({ size: 1, panes: row.panes.map(pane => ({ ...pane, size: 1 })) })) }
}

/**
 * Shares after the edge between `index` and `index + 1` moves to `at` (0 to 1 of
 * the whole). The two neighbours trade room; neither goes under the least.
 */
function split(sizes: number[], index: number, at: number): number[] {
  const total = sizes.reduce((sum, size) => sum + size, 0)
  if (total <= 0) return sizes
  const before = sizes.slice(0, index).reduce((sum, size) => sum + size, 0)
  const pair = sizes[index]! + sizes[index + 1]!
  const least = Math.min(pair / 2, (LEAST * total) / sizes.length)
  const first = Math.min(pair - least, Math.max(least, at * total - before))
  return sizes.map((size, i) => (i === index ? first : i === index + 1 ? pair - first : size))
}

/** A layout read back from storage, if it is one. Anything else is no layout. */
export function parse(saved: unknown): Layout {
  if (!saved || typeof saved !== 'object' || !Array.isArray((saved as Layout).rows)) return EMPTY
  const seen = new Set<string>()
  const share = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : 1)
  const rows: Row[] = []
  for (const row of (saved as Layout).rows) {
    if (!row || !Array.isArray(row.panes)) return EMPTY
    const panes: Pane[] = []
    for (const pane of row.panes) {
      if (!pane || typeof pane.id !== 'string' || !pane.id || seen.has(pane.id)) return EMPTY
      seen.add(pane.id)
      panes.push({ id: pane.id, size: share(pane.size) })
    }
    if (panes.length > 0) rows.push({ size: share(row.size), panes })
  }
  return { rows }
}

/** Without the panes of desks that are no longer there. */
export function only(layout: Layout, alive: (id: string) => boolean): Layout {
  return ids(layout).reduce((current, id) => (alive(id) ? current : close(current, id)), layout)
}
