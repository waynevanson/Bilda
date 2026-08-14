import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const CACHE_DIR = join(tmpdir(), 'marp-mermaid-cache')

function findChromium() {
  if (process.env.PUPPETEER_EXECUTABLE_PATH) {
    return process.env.PUPPETEER_EXECUTABLE_PATH
  }

  try {
    const result = spawnSync(
      'command -v chromium || command -v google-chrome-stable || command -v google-chrome || command -v chrome',
      { encoding: 'utf8', shell: true },
    )
    return result.stdout.trim()
  } catch {
    return null
  }
}

function renderMermaidToSvg(source) {
  const chromium = findChromium()

  if (!chromium) {
    throw new Error(
      'Marp mermaid plugin requires a Chromium-based browser. Set PUPPETEER_EXECUTABLE_PATH or install chromium/google-chrome.',
    )
  }

  const hash = createHash('sha256').update(source).digest('hex')
  const cacheFile = join(CACHE_DIR, `${hash}.svg`)

  try {
    return readFileSync(cacheFile, 'utf8')
  } catch {
    // cache miss
  }

  mkdirSync(CACHE_DIR, { recursive: true })

  const inputFile = join(CACHE_DIR, `${hash}.mmd`)
  writeFileSync(inputFile, source)

  const mmdc = resolve(
    dirname(fileURLToPath(import.meta.url)),
    '../node_modules/.bin/mmdc',
  )
  const env = { ...process.env, PUPPETEER_EXECUTABLE_PATH: chromium }

  try {
    const result = spawnSync(
      mmdc,
      ['-i', inputFile, '-o', cacheFile, '-b', 'transparent', '-q'],
      { env, stdio: 'pipe' },
    )

    if (result.error) throw result.error
    if (result.status !== 0) {
      throw new Error(result.stderr.toString() || result.stdout.toString())
    }
  } catch (err) {
    throw new Error(`Failed to render mermaid diagram: ${err.message}`)
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

export default function marpMermaid(md) {
  md.core.ruler.after('block', 'marp_mermaid', (state) => {
    for (let i = 0; i < state.tokens.length; i += 1) {
      const token = state.tokens[i]

      if (token.type !== 'fence') continue

      const lang = token.info.trim().split(/\s+/)[0]
      if (lang !== 'mermaid') continue

      const svg = cleanSvg(renderMermaidToSvg(token.content))
      const htmlToken = new state.Token('html_block', '', 0)
      htmlToken.content = `<div class="mermaid-diagram">${svg}</div>\n`
      htmlToken.map = token.map

      state.tokens.splice(i, 1, htmlToken)
    }
  })
}
