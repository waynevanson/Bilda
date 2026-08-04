import mermaid from 'mermaid'
import diagram from '../state.mermaid?raw'

const app = document.getElementById('app')

async function render(src = diagram) {
  try {
    const { svg } = await mermaid.render('diagram', src)
    app.innerHTML = svg
  } catch (err) {
    app.innerHTML = `<pre style="color:red">${err.message}</pre>`
  }
}

mermaid.initialize({ startOnLoad: false })
render()

if (import.meta.hot) {
  import.meta.hot.accept('../state.mermaid?raw', (mod) => {
    render(mod.default)
  })
}
