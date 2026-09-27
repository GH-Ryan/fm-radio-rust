interface TitlebarProps {
  onClose: () => void
  onMinimize: () => void
}

export function Titlebar({ onClose, onMinimize }: TitlebarProps) {
  return (
    <div className="titlebar">
      <div className="titlebar-left">
        <svg className="app-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9c2-2 5.1-2 7.1 0 1.2 1.2 2 2.9 2 4.6 0 3.6-2.9 6.5-6.5 6.5-1.7 0-3.4-.8-4.6-2z" />
          <path d="M7.8 16.2c-1.1 1.1-1.7 2.6-1.7 4.2 0 3.1 2.5 5.6 5.6 5.6 1.6 0 3.1-.6 4.2-1.7" />
          <path d="M11 5a6 6 0 0 1 6 6c0 1.6-.6 3.1-1.7 4.2" />
          <circle cx="12" cy="12" r="2" />
        </svg>
        <span className="app-title">FM 电台</span>
      </div>
      <div className="titlebar-right">
        <button className="titlebar-btn" onClick={onMinimize} title="最小化">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M5 12h14" />
          </svg>
        </button>
        <button className="titlebar-btn close" onClick={onClose} title="关闭">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  )
}