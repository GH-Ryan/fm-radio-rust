import { backendErrors, backendErrorsEn } from './backendErrors'
import { enUS } from './en-US'
import { zhCN } from './zh-CN'
import type { TranslationKey, Translations } from './zh-CN'

export type { TranslationKey, Translations }
export type Locale = 'zh-CN' | 'en-US'

/**
 * Locales the app ships.
 *
 * `zh-CN` first on purpose: it is what the UI falls back to when the system
 * reports a language this app does not have, and a partially translated
 * interface is worse than one that is consistently in the author's language.
 */
export const LOCALES: readonly Locale[] = ['zh-CN', 'en-US']

export const DEFAULT_LOCALE: Locale = 'zh-CN'

const CATALOGUES: Record<Locale, Translations> = {
  'zh-CN': zhCN,
  'en-US': enUS,
}

export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value)
}

/**
 * Picks the closest supported locale for a system language tag.
 *
 * The match is on the primary subtag, so `en-GB` and `en-US` both land on
 * `en-US`, and `zh-Hans-CN` lands on `zh-CN`. An exact match wins when there
 * is one.
 */
export function resolveLocale(systemTag: string): Locale {
  if (!systemTag) return DEFAULT_LOCALE

  const exact = CATALOGUES_KEYS.find((l) => l.toLowerCase() === systemTag.toLowerCase())
  if (exact) return exact

  const primary = systemTag.toLowerCase().split(/[-_]/)[0]
  const partial = CATALOGUES_KEYS.find((l) => l.toLowerCase().split('-')[0] === primary)
  return partial ?? DEFAULT_LOCALE
}

const CATALOGUES_KEYS = Object.keys(CATALOGUES) as Locale[]

export type TranslateParams = Record<string, string | number>

/**
 * Looks up a string and substitutes {placeholders}.
 *
 * A missing key returns the key itself rather than an empty string: a blank
 * button is invisible, and a visible `playPauseLabel` in the UI is a bug report
 * that names itself.
 */
export function translate(
  locale: Locale,
  key: TranslationKey,
  params?: TranslateParams,
): string {
  const template = CATALOGUES[locale]?.[key] ?? key

  if (!params) return template

  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  )
}
/**
 * The message to show for a backend failure.
 *
 * Looks the code up in the reader's language first. A code with no entry -- one
 * added on the backend after this build shipped -- falls through to the
 * message the backend sent, which is better than showing a raw code.
 */
export function translateBackendError(
  locale: Locale,
  code: string,
  message: string,
  detail?: string,
): string {
  const table = locale === 'en-US' ? backendErrorsEn : backendErrors
  const entry = (table as Record<string, string | ((d?: string) => string)>)[code]

  if (entry === undefined) return message

  if (typeof entry === 'function') return entry(detail)

  return entry.replace(/\{detail\}/g, detail ?? '')
}