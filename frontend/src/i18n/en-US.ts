import type { Translations } from './zh-CN'

// English.
//
// `satisfies` is what makes a missing or misspelled key a compile error rather
// than a key rendered in the UI at runtime. The value type is widened to
// string here on purpose: a literal type would force the English copy to match
// the Chinese character for character, which is not the constraint that
// matters -- having the key is.
export const enUS = {
  // ---- Generic ----
  close: 'Close',
  unknown: 'Unknown',

  // ---- Title bar and tabs ----
  appTitle: 'FM Radio',
  tabStations: 'Stations',
  tabFavorites: 'Favourites',

  // ---- Search and list ----
  searchPlaceholder: 'Search stations…',
  filterFavoritesPlaceholder: 'Filter favourites…',
  stationsCount: '{count}',

  emptyStationsTitle: 'No stations found',
  emptyStationsHint: 'Try a different search, or check the connection',
  emptyFavoritesHint: 'No favourites yet. Tap the star on a station to add one.',
  favoriteCountBadge: '{count}',

  // ---- Player ----
  playerRegionLabel: 'Player controls',
  unknownStation: 'Unknown station',
  play: 'Play',
  pause: 'Pause',
  stop: 'Stop',
  volume: 'Volume',
  randomFromBrowse: 'Shuffle stations',
  randomFromFavorites: 'Shuffle favourites',
  favorite: 'Favourite',
  unfavorite: 'Remove from favourites',
  mute: 'Mute',
  unmute: 'Unmute',
  cardPlay: 'Play',
  cardPause: 'Pause',

  // ---- Station metadata ----
  unknownCodec: 'Unknown format',

  // ---- Errors raised by the UI ----
  loadFailed: 'Could not load: {error}',
  searchFailed: 'Search failed: {error}',
  loadFavoritesFailed: 'Could not load favourites: {error}',
  playFailed: 'Playback failed',
  actionFailed: 'Action failed',
  stopFailed: 'Could not stop: {error}',
  randomFailed: 'Shuffle failed',
  volumeFailed: 'Could not change volume: {error}',
  listenFailed: 'Lost track of playback: {error}',
  superseded: 'Replaced by a newer request',
  closeFailed: 'Could not close: {error}',

  // ---- Errors from the backend ----
  backendError: '{message}',

  // ---- Playback status ----
  backendConnecting: 'Connecting',
} satisfies Translations