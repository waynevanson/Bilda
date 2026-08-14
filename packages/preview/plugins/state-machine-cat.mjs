import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const CACHE_DIR = join(tmpdir(), 'marp-state-machine-cat-cache')

function renderStateMachineCatToSvg(source) {
  const hash = createHash('sha256').update(source).digest('hex')
  const cacheFile = join(CACHE_DIR, `${hash}.svg`)

  try {
    return readFileSync(cacheFile, 'utf8')
  } catch {
    // cache miss
  }

  mkdirSync(CACHE_DIR, { recursive: true })

  const inputFile = join(CACHE_DIR, `${hash}.smcat`)
  writeFileSync(inputFile, source)

  const smcat = resolve(
    dirname(fileURLToPath(import.meta.url)),
    '../node_modules/.bin/smcat',
  )

  try {
    const result = spawnSync(
      smcat,
      ['-T', 'svg', '-o', cacheFile, inputFile],
      { stdio: 'pipe' },
    )

    if (result.error) throw result.error
    if (result.status !== 0) {
      throw new Error(result.stderr.toString() || result.stdout.toString())
    }
  } catch (err) {
    throw new Error(`Failed to render state-machine-cat diagram: ${err.message}`)
  } finally {
    rmSync(inputFile, { force: true })
  }

  return readFileSync(cacheFile, 'utf8')
}

function cleanSvg(svg) {
  return svg
    .replace(/^<\?xml[^?]*\?>\s*/, '')
    .replace(/<!DOCTYPE[^>]*>/i, '')
    .trim()
}

export default function marpStateMachineCat(md) {
  md.core.ruler.after('block', 'marp_state_machine_cat', (state) => {
    for (let i = 0; i < state.tokens.length; i += 1) {
      const token = state.tokens[i]

      if (token.type !== 'fence') continue

      const lang = token.info.trim().split(/\s+/)[0]
      if (lang !== 'smcat') continue

      const svg = cleanSvg(renderStateMachineCatToSvg(token.content))
      const htmlToken = new state.Token('html_block', '', 0)
      htmlToken.content = `<div class="state-machine-cat-diagram">${svg}</div>\n`
      htmlToken.map = token.map

      state.tokens.splice(i, 1, htmlToken)
    }
  })
}
