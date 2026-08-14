import { Marp } from '@marp-team/marp-core'
import elementTransitions from './plugins/element-transitions.mjs'
import mermaid from './plugins/mermaid.mjs'

export default class Engine extends Marp {
  constructor(...args) {
    super(...args)
    this.use(elementTransitions)
    this.use(mermaid)
  }
}
