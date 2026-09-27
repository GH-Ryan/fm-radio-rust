export interface StationView {
  key: string
  uuid: string
  name: string
  country: string
  countryCode: string
  language: string
  codec: string
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