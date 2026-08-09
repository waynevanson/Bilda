const TRANSITIONS = new Set([
  'fade',
  'slide',
  'convex',
  'concave',
  'zoom',
  'implode',
  'none',
])

function parseTransition(value) {
  if (value === true) return { name: 'fade', duration: '' }

  const str = String(value).trim()
  const [name, ...rest] = str.split(/\s+/)

  return {
    name: TRANSITIONS.has(name) ? name : 'fade',
    duration: rest.join(' '),
  }
}

function addClass(token, cls) {
  const idx = token.attrIndex('class')

  if (idx < 0) {
    token.attrPush(['class', cls])
  } else {
    const attr = token.attrs[idx]
    const classes = new Set(attr[1].split(/\s+/))
    classes.add(cls)
    attr[1] = [...classes].join(' ')
  }
}

function setDuration(token, duration) {
  if (!duration) return

  const idx = token.attrIndex('style')
  const declaration = `--fragment-duration: ${duration};`

  if (idx < 0) {
    token.attrPush(['style', declaration])
  } else {
    const attr = token.attrs[idx]
    attr[1] += attr[1].endsWith(';') ? declaration : `;${declaration}`
  }
}

function applyClass(token, transition) {
  if (transition.name && transition.name !== 'none') {
    addClass(token, `transition-${transition.name}`)
  }

  setDuration(token, transition.duration)
}

function applyFragment(token, transition, index) {
  token.attrSet('data-marpit-fragment', index)
  applyClass(token, transition)
}

function findInlineFragment(inlineToken) {
  for (const child of inlineToken.children || []) {
    if (child.type === 'marpit_comment') {
      const directives = child.meta?.marpitParsedDirectives || {}
      const value = directives.fragment ?? directives['element-transition']

      if (value !== undefined) return parseTransition(value)
    }
  }

  return null
}

function generateCSS() {
  return `
[data-marpit-fragment] {
  opacity: 0;
  visibility: hidden;
  transition: opacity var(--fragment-duration, 0.4s) ease,
              transform var(--fragment-duration, 0.4s) ease,
              filter var(--fragment-duration, 0.4s) ease;
}

[data-marpit-fragment][data-bespoke-marp-fragment="active"] {
  opacity: 1;
  visibility: visible;
  transform: none;
  filter: none;
}

[data-marpit-fragment].transition-fade { opacity: 0; }
[data-marpit-fragment].transition-slide { transform: translateX(40px); }
[data-marpit-fragment].transition-convex { transform: perspective(600px) rotateY(-25deg) translateX(40px); }
[data-marpit-fragment].transition-concave { transform: perspective(600px) rotateY(25deg) translateX(-40px); }
[data-marpit-fragment].transition-zoom { transform: scale(0.5); }
[data-marpit-fragment].transition-implode { transform: scale(1.5); }

@media print {
  [data-marpit-fragment] {
    opacity: 1 !important;
    visibility: visible !important;
    transform: none !important;
    filter: none !important;
  }
}

@media (prefers-reduced-motion: reduce) {
  [data-marpit-fragment] {
    transition: none !important;
  }
}
  `.trim()
}

export default function elementTransitions(md) {
  const marpit = md.marpit

  marpit.customDirectives.global['element-transition'] = (value) => ({
    'element-transition': value,
  })

  marpit.customDirectives.local['element-transition'] = (value) => ({
    'element-transition': value,
  })

  let initialized = false

  md.core.ruler.after('marpit_apply_fragment', 'marp_element_transitions', (state) => {
    if (state.inlineMode) return

    if (!initialized) {
      initialized = true
      marpit.lastStyles = marpit.lastStyles || []
      marpit.lastStyles.push(generateCSS())
    }

    let currentSlide = null
    let fragmentCount = 0
    let slideTransition = null
    let pending = null
    let currentBlockOpen = null
    let listItem = null

    for (let i = 0; i < state.tokens.length; i += 1) {
      const token = state.tokens[i]

      if (token.type === 'marpit_slide_open') {
        currentSlide = token
        fragmentCount = 0

        const directives = token.meta?.marpitDirectives || {}
        const value = directives['element-transition']
        slideTransition = value !== undefined ? parseTransition(value) : null
        pending = null
        currentBlockOpen = null
        listItem = null
        continue
      }

      if (token.type === 'marpit_slide_close') {
        if (currentSlide && fragmentCount > 0) {
          currentSlide.attrSet('data-marpit-fragments', fragmentCount)
        }

        currentSlide = null
        fragmentCount = 0
        slideTransition = null
        pending = null
        currentBlockOpen = null
        listItem = null
        continue
      }

      const directives = token.meta?.marpitParsedDirectives || {}
      const value = directives.fragment ?? directives['element-transition']
      if (value !== undefined) {
        pending = parseTransition(value)
      }

      if (token.type === 'bullet_list_open' || token.type === 'ordered_list_open') {
        if (pending) pending.fromList = true
        continue
      }

      if (token.type === 'bullet_list_close' || token.type === 'ordered_list_close') {
        if (pending?.fromList) pending = null
        continue
      }

      if (token.type === 'paragraph_open' || /^h[1-6]_open$/.test(token.type)) {
        currentBlockOpen = token
      }

      if (token.type === 'paragraph_close' || /^h[1-6]_close$/.test(token.type)) {
        currentBlockOpen = null
      }

      if (token.type === 'list_item_open') {
        listItem = token

        if (pending) {
          if (token.attrGet('data-marpit-fragment')) {
            applyClass(token, pending)
          } else {
            applyFragment(token, pending, ++fragmentCount)
          }
        } else if (slideTransition) {
          if (token.attrGet('data-marpit-fragment')) {
            applyClass(token, slideTransition)
          } else {
            applyFragment(token, slideTransition, ++fragmentCount)
          }
        }

        continue
      }

      if (token.type === 'list_item_close') {
        listItem = null
        continue
      }

      if (!listItem && (token.type === 'paragraph_open' || /^h[1-6]_open$/.test(token.type))) {
        if (pending) {
          if (token.attrGet('data-marpit-fragment')) {
            applyClass(token, pending)
          } else {
            applyFragment(token, pending, ++fragmentCount)
          }

          pending = null
          continue
        }

        if (slideTransition && token.attrGet('data-marpit-fragment')) {
          applyClass(token, slideTransition)
          continue
        }
      }

      if (token.type === 'inline') {
        const inline = findInlineFragment(token)

        if (inline) {
          if (listItem) {
            if (listItem.attrGet('data-marpit-fragment')) {
              applyClass(listItem, inline)
            } else {
              applyFragment(listItem, inline, ++fragmentCount)
            }
          } else if (currentBlockOpen) {
            if (currentBlockOpen.attrGet('data-marpit-fragment')) {
              applyClass(currentBlockOpen, inline)
            } else {
              applyFragment(currentBlockOpen, inline, ++fragmentCount)
            }
          }
        }
      }
    }
  })
}
