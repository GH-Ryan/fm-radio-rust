// Normalises whatever a rejected invoke() hands back into something the UI can
// act on.
//
// Tauri serialises a command's Err into the rejection, and the backend now
// sends an object carrying a stable `code` alongside a message written in the
// app's source language. Reading the code is what makes it possible to branch
// on what went wrong and to translate it; reading the message is the fallback
// for anything that arrives without a code.
//
// The old code matched the Chinese wording of the message to recognise a
// superseded request, which stops working the moment the copy is reworded or
// the backend speaks another language.
import type { AppError } from '../types'

export const BACKEND_ERROR_CODES = [
  'output_unavailable',
  'connection_failed',
  'unsupported_format',
  'format_not_supported',
  'decode_error',
  'stream_ended',
  'cancelled',
  'nothing_playing',
  'empty_list',
  'unknown_list',
  'network',
  'json',
  'all_mirrors_failed',
  'no_mirrors',
  'http',
  'io',
  'url_parse',
  'config_dir_unavailable',
  'superseded',
] as const

export type BackendErrorCode = (typeof BACKEND_ERROR_CODES)[number]

export function isSuperseded(e: unknown): boolean {
  return readError(e)?.code === 'superseded'
}

/** The message to show when there is nothing better to say. */
export function errorMessage(e: unknown): string {
  const err = readError(e)
  return err?.message ?? String(e)
}

/**
 * Pulls the structured error out of whatever shape it arrived in.
 *
 * Tauri may deliver the AppError object directly, or wrap it, depending on
 * whether the rejection crosses a serialisation boundary. Both are handled
 * rather than assumed, because the failure mode when this is wrong is an error
 * message the user sees instead of the one they were supposed to get.
 */
function readError(e: unknown): AppError | null {
  if (typeof e === 'string') return null
  if (e && typeof e === 'object') {
    const candidate = e as Partial<AppError>
    if (typeof candidate.code === 'string' && typeof candidate.message === 'string') {
      return { code: candidate.code, message: candidate.message, detail: candidate.detail }
    }
    // A wrapper that holds the real error one level down.
    if ('error' in candidate) return readError(candidate.error)
  }
  return null
}