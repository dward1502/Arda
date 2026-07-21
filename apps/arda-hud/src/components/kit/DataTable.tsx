// sigil: REPAIR
import type { ReactNode } from 'react'

interface DataTableColumn<T> {
  key: keyof T & string
  header: string
  render?: (value: T[keyof T], row: T) => ReactNode
  className?: string
}

interface DataTableProps<T> {
  columns: DataTableColumn<T>[]
  data: T[]
  keyField: keyof T & string
  onRowClick?: (row: T) => void
  className?: string
}

export default function DataTable<T extends Record<string, unknown>>({
  columns,
  data,
  keyField,
  onRowClick,
  className = '',
}: DataTableProps<T>) {
  return (
    <table
      className={`w-full border-collapse font-['Share_Tech_Mono'] text-xs ${className}`}
    >
      <thead>
        <tr className="border-b border-[#ff6b3566]">
          {columns.map((col) => (
            <th
              key={col.key}
              className={`px-3 py-2.5 text-left font-['Orbitron'] text-[8px] font-bold uppercase tracking-[0.18em] text-[#ff6b35] ${col.className ?? ''}`}
            >
              {col.header}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {data.map((row) => (
          <tr
            key={String(row[keyField])}
            onClick={onRowClick ? () => onRowClick(row) : undefined}
            className={`group border-b border-[#9b59b611] transition-colors duration-150 hover:bg-[#ff6b3522] ${onRowClick ? 'cursor-pointer' : ''}`}
          >
            {columns.map((col, colIdx) => (
              <td
                key={col.key}
                className={`px-3 py-2.5 text-[#e8e8f066] transition-colors duration-150 group-hover:text-[#e8e8f0] ${colIdx === 0 ? 'group-hover:border-l-2 group-hover:border-l-[#ff6b35]' : ''} ${col.className ?? ''}`}
              >
                {col.render
                  ? col.render(row[col.key], row)
                  : String(row[col.key] ?? '')}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  )
}
