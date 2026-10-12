import { useI18n } from '../i18n/useI18n'

interface TitlebarProps {
  onClose: () => void
}

export function Titlebar({ onClose }: TitlebarProps) {
  const { t } = useI18n()

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-left" data-tauri-drag-region>
        <svg className="app-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <rect x="2.75" y="9.25" width="18.5" height="11.5" rx="2.25" strokeWidth="1.5" />
          <path d="M6 13h3.5M6 16h3.5M6 19h3.5" strokeWidth="1.2" />
          <circle cx="15.75" cy="16.25" r="2.5" strokeWidth="1.5" />
          <path d="M14.25 14.75l3 3" strokeWidth="1.2" />
          <path d="M7 9.25L17.5 4.25" strokeWidth="1.4" />
        </svg>
        <span className="app-title">{t('appTitle')}</span>
      </div>
      <div className="titlebar-right">
        <button className="titlebar-btn close" onClick={onClose} title={t('close')} aria-label={t('close')}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  )
}