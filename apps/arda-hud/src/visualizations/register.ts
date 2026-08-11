import { registerTemplate } from './registry'
import { StructuredTemplate } from './templates/StructuredTemplate'

registerTemplate({
  id: 'structured',
  label: 'Structured',
  description: 'Default structured layout',
  component: StructuredTemplate,
})