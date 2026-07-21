// sigil: REPAIR
interface LineListItem {
  label: string
  value: string
}

interface LineListProps {
  items: LineListItem[]
}

export default function LineList({ items }: LineListProps) {
  return (
    <div className="line-list">
      {items.map((item) => (
        <div className="line-list__row" key={`${item.label}-${item.value}`}>
          <span>{item.label}</span>
          <strong>{item.value}</strong>
        </div>
      ))}
    </div>
  )
}
