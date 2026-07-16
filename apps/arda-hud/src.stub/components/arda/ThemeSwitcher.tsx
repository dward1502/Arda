// sigil: REPAIR
interface ThemeOption {
  id: string
  label: string
}

interface ThemeSwitcherProps {
  themes: ThemeOption[]
  activeTheme: string
  onChange: (themeId: string) => void
}

export default function ThemeSwitcher({ themes, activeTheme, onChange }: ThemeSwitcherProps) {
  return (
    <div className="theme-switcher">
      {themes.map((theme) => (
        <button
          key={theme.id}
          type="button"
          className={theme.id === activeTheme ? 'theme-switcher__button is-active' : 'theme-switcher__button'}
          onClick={() => onChange(theme.id)}
        >
          {theme.label}
        </button>
      ))}
    </div>
  )
}

