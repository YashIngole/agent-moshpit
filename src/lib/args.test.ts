import { describe, expect, it } from 'vitest'
import { typed, words } from './args'

describe('extra arguments, typed as on a command line', () => {
  it('parts words at spaces and line ends', () => {
    expect(words('--remote-control --fallback-model sonnet')).toEqual(['--remote-control', '--fallback-model', 'sonnet'])
    expect(words('  --enable   one_feature\n--verbose\t-c x=1 \r\n')).toEqual(['--enable', 'one_feature', '--verbose', '-c', 'x=1'])
    expect(words('')).toEqual([])
    expect(words('   \n ')).toEqual([])
  })

  it('keeps what is in quotes together, without the quotes', () => {
    expect(words('--name "two words" --x')).toEqual(['--name', 'two words', '--x'])
    expect(words("--name 'two words'")).toEqual(['--name', 'two words'])
    // A value joined to its flag stays joined; inside one kind of quote the other is just a character.
    expect(words('-c model_verbosity="low" --say \'he said "hi"\'')).toEqual(['-c', 'model_verbosity=low', '--say', 'he said "hi"'])
    expect(words(`--say "it's fine"`)).toEqual(['--say', "it's fine"])
    // A quote left open keeps the rest.
    expect(words('--name "left open here')).toEqual(['--name', 'left open here'])
    // Nothing in quotes is no word: a program is never handed an empty one.
    expect(words('--x "" \'\'')).toEqual(['--x'])
  })

  it('expands nothing: these are not read by a shell', () => {
    expect(words('--x $HOME %PATH% `id` a&b c|d ~ * \\n')).toEqual(['--x', '$HOME', '%PATH%', '`id`', 'a&b', 'c|d', '~', '*', '\\n'])
  })

  it('writes words back as they would be typed, to be read the same again', () => {
    for (const list of [['--remote-control'], ['--name', 'two words'], ['--say', 'he said "hi"'], ['--say', "it's fine"], ['-c', 'model="o3"'], ['--say', `he said "it's"`, '--x'], ['"'], []]) {
      expect(words(typed(list))).toEqual(list)
    }
    expect(typed(['--name', 'two words', '--x'])).toBe('--name "two words" --x')
  })
})
