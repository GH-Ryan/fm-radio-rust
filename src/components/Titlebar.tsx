import { useEffect, useState } from 'react'
import { type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from '../i18n/useI18n'

interface TitlebarProps {
  onClose: () => void
  onMinimize: () => void
  onToggleMaximize: () => void
  onWindowError: (error: unknown) => void
}

export function Titlebar({ onClose, onMinimize, onToggleMaximize, onWindowError }: TitlebarProps) {
  const { t } = useI18n()
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    const appWindow = getCurrentWindow()
    let active = true
    let unlisten: UnlistenFn | undefined

    const updateMaximized = async () => {
      try {
        const value = await appWindow.isMaximized()
        if (active) setMaximized(value)
      } catch (error) {
        if (active) onWindowError(error)
      }
    }

    void updateMaximized()
    void appWindow
      .onResized(() => {
        void updateMaximized()
      })
      .then((stopListening) => {
        if (active) unlisten = stopListening
        else stopListening()
      })
      .catch((error: unknown) => {
        if (active) onWindowError(error)
      })

    return () => {
      active = false
      unlisten?.()
    }
  }, [onWindowError])

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
        <button className="titlebar-btn" onClick={onMinimize} title={t('minimize')} aria-label={t('minimize')}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
            <path d="M5 12h14" />
          </svg>
        </button>
        <button
          className="titlebar-btn"
          onClick={onToggleMaximize}
          title={t(maximized ? 'restore' : 'maximize')}
          aria-label={t(maximized ? 'restore' : 'maximize')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
            {maximized ? (
              <path d="M8 5H5v14h14v-3M9 15h10V5H9z" />
            ) : (
              <path d="M5 5h14v14H5z" />
            )}
          </svg>
        </button>
        <button className="titlebar-btn close" onClick={onClose} title={t('close')} aria-label={t('close')}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  )
}