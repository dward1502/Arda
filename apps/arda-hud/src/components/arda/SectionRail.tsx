// sigil: REPAIR
import type { ArdaSection } from '../../lib/ardaSource'

interface SectionRailProps {
  sections: ArdaSection[]
  activeSectionId: string | null
  onSelect: (sectionId: string) => void
}

export default function SectionRail({ sections, activeSectionId, onSelect }: SectionRailProps) {
  return (
    <nav className="section-rail" aria-label="ARDA sections">
      {sections.map((section) => {
        const active = section.id === activeSectionId
        return (
          <button
            key={section.id}
            type="button"
            className={active ? 'section-rail__item is-active' : 'section-rail__item'}
            onClick={() => onSelect(section.id)}
          >
            <span className="section-rail__title">{section.title}</span>
            <span className="section-rail__meta">{section.owner}</span>
          </button>
        )
      })}
    </nav>
  )
}
