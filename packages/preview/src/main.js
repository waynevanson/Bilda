import { Marp } from '@marp-team/marp-core'
import slides from '../PRESENTATION.md?raw'

const app = document.getElementById('app')

function render(src = slides) {
  try {
    const marp = new Marp()
    const { html, css } = marp.render(src)
    app.innerHTML = `<style>${css}</style>${html}`
  } catch (err) {
    app.innerHTML = `<pre style="color:red">${err.message}</pre>`
  }
}

render()

if (import.meta.hot) {
  import.meta.hot.accept('../PRESENTATION.md?raw', (mod) => {
    render(mod.default)
  })
}
