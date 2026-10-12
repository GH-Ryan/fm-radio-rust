import { useCallback, useEffect, useMemo, useState } from 'react'
import { DEFAULT_LOCALE, LOCALES, isLocale, resolveLocale, translate } from './index'
import type { Locale, TranslateParams, TranslationKey } from './index'

/**
 * The user's chosen language, or null when they have not chosen.
 *
 * Persisted separately from the resolved locale on purpose: the system
 * language is a default, and an explicit choice must survive it changing --
 * someone who picked English while running a Chinese Windows should not have
 * it flip back on the next boot.
 */
const STORAGE_KEY = 'fmradio.locale'

function readStored(): Locale | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    return raw && isLocale(raw) ? raw : null
  } catch {
    // Private browsing, or storage disabled. Not worth failing over; the app
    // just follows the system each time.
    return null
  }
}

function writeStored(locale: Locale | null): void {
  try {
    if (locale) window.localStorage.setItem(STORAGE_KEY, locale)
    else window.localStorage.removeItem(STORAGE_KEY)
  } catch {
    // Same reasoning as above.
  }
}

/**
 * The system language, as the browser sees it.
 *
 * navigator.language follows the OS setting and carries the region, which is
 * what resolveLocale matches on. It is empty in a few webview configurations,
 * in which case the default stands.
 */
function systemLocale(): Locale {
  const tag = typeof navigator !== 'undefined' ? navigator.language : ''
  return resolveLocale(tag ?? '')
}

export interface I18n {
  locale: Locale
  /** Changes the language and remembers the choice. */
  setLocale: (next: Locale) => void
  /** Goes back to following the system language. */
  clearOverride: () => void
  /** True when an explicit choice is in effect. */
  isOverridden: boolean
  t: (key: TranslationKey, params?: TranslateParams) => string
}

export function useI18n(): I18n {
  const [override, setOverride] = useState<Locale | null>(readStored)
  const [system, setSystem] = useState<Locale>(systemLocale)

  // The system language can change while the app is running -- a user who
  // changes it in Settings without restarting would otherwise be stuck.
  useEffect(() => {
    if (typeof navigator === 'undefined') return

    const handler = () => setSystem(systemLocale())
    window.addEventListener('languagechange', handler)
    return () => window.removeEventListener('languagechange', handler)
  }, [])

  const locale = override ?? system

  const setLocale = useCallback((next: Locale) => {
    setOverride(next)
    writeStored(next)
  }, [])

  const clearOverride = useCallback(() => {
    setOverride(null)
    writeStored(null)
  }, [])

  const t = useCallback(
    (key: TranslationKey, params?: TranslateParams) => translate(locale, key, params),
    [locale],
  )

  return useMemo(
    () => ({
      locale,
      setLocale,
      clearOverride,
      isOverridden: override !== null,
      t,
    }),
    [locale, setLocale, clearOverride, t, override],
  )
}

export { LOCALES, DEFAULT_LOCALE }
export type { Locale }