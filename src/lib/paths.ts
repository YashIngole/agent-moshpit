// File paths printed in a terminal, which Ctrl and a click opens in the editor.

/** A path: a drive or a ./ or folders, then a name with an extension, then perhaps :line and :column. */
const PATH = /(?:[A-Za-z]:[\\/]|\.{1,2}[\\/]|[\\/])?(?:[\w.@-]+[\\/])*[\w@-][\w.@-]*\.([A-Za-z][A-Za-z0-9]{0,7})(?::(\d+)(?::\d+)?)?/g
const WEB = /\b[a-z][a-z0-9+.-]*:\/\/\S+/gi
/** Extensions that say "a file" by themselves, without a folder or a line to go on. */
const CODE = new Set(
  'ts tsx js jsx mjs cjs json md mdx rs py go java kt rb php cs cpp cc c h hpp css scss sass less html svelte vue astro yml yaml toml lock sh ps1 bat sql txt env ini cfg conf xml gradle swift dart lua ex exs erl zig'.split(' ')
)

export interface FoundPath {
  /** Where it starts in the line, counting from zero, and how long it is. */
  start: number
  length: number
  path: string
  line: number | null
}

/** The file paths in one line of a terminal. Web addresses are not paths. */
export function findPaths(text: string): FoundPath[] {
  const web = [...text.matchAll(WEB)].map(m => [m.index!, m.index! + m[0].length] as const)
  const found: FoundPath[] = []
  for (const match of text.matchAll(PATH)) {
    const [whole, extension = '', line] = match
    const start = match.index!
    if (web.some(([from, to]) => start < to && start + whole.length > from)) continue
    const path = whole.replace(/(?::\d+){1,2}$/, '')
    // "Node.js" and "e.g" are words; a folder, a line number or a code extension makes it a path.
    const placed = /[\\/]/.test(path) || line !== undefined
    // "Node.js", "Vue.js": names of things, not files, unless a folder or a line says otherwise.
    const named = /^[A-Z][a-z]+\.js$/.test(path)
    if (!placed && (named || !CODE.has(extension.toLowerCase()))) continue
    found.push({ start, length: whole.length, path, line: line === undefined ? null : Number(line) })
  }
  return found
}
