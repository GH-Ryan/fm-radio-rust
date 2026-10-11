export interface StationView {
  key: string
  uuid: string
  name: string
  country: string
  countryCode: string
  language: string
  codec: string
  /**
   * Whether the catalogue named a codec at all.
   *
   * Carried separately because codec holds the label to display, and the
   * label for a station that named none is a sentence rather than data.
   */
  codecKnown: boolean
  bitrate: number
  votes: number
  tagList: string[]
  streamURL: string
  homepage: string
  favicon: string
  displayName: string
  meta: string
  genre: string
  favorite: boolean
}

export interface PlayerView {
  playing: boolean
  playingKey: string | null
  paused: boolean
  name: string
  title: string
  backend: string
  error: string | null
  errorSticky: boolean
  volume: number
  station: StationView | null
}
/**
 * A failure reported by the backend.
 *
 * `code` is stable and is what the UI branches on. `message` is written in the
 * app's source language; the UI substitutes its own translation for the code
 * when it has one, and falls back to this otherwise.
 */
export interface AppError {
  code: string
  message: string
  detail?: string
}