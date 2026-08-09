import { Marp } from '@marp-team/marp-core'
import elementTransitions from './plugins/element-transitions.mjs'

export default class Engine extends Marp {
  constructor(...args) {
    super(...args)
    this.use(elementTransitions)
  }
}
