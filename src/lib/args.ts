// Words typed as on a command line, and the words a program is given.
//
// Only what a person needs to hand a program a value with a space in it: words are
// parted by spaces and line ends, and quotes keep what is inside them in one word.
// Nothing is expanded and nothing is run: these are never read by a shell.

/**
 * The words in what was typed. A pair of quotes, double or single, keeps its inside
 * together and is not itself part of the word; inside one kind, the other is a
 * character like any other. A quote left open keeps everything after it.
 */
export function words(typed: string): string[] {
  const found: string[] = []
  let word = ''
  /** Whether a word has begun, even an empty one in quotes. */
  let begun = false
  let quote = ''
  for (const c of typed) {
    if (quote) {
      if (c === quote) quote = ''
      else word += c
    } else if (c === '"' || c === "'") {
      quote = c
      begun = true
    } else if (/\s/.test(c)) {
      if (begun && word) found.push(word)
      word = ''
      begun = false
    } else {
      word += c
      begun = true
    }
  }
  if (begun && word) found.push(word)
  return found
}

/** The words as they would be typed: in quotes where one has a space or a quote in it, so that `words` reads them back the same. */
export function typed(list: string[]): string {
  return list.map(quoted).join(' ')
}

function quoted(word: string): string {
  if (!/[\s"']/.test(word)) return word
  if (!word.includes('"')) return `"${word}"`
  if (!word.includes("'")) return `'${word}'`
  // Both kinds in one word: each stretch in double quotes, and each double quote in single ones.
  return word
    .split('"')
    .map(part => (part ? `"${part}"` : ''))
    .join(`'"'`)
}
