import React, { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { StationView, PlayerView } from './types'
import { Titlebar } from './components/Titlebar'
import { StationGrid } from './components/StationGrid'
import { PlayerBar } from './components/PlayerBar'
import { Toast } from './components/Toast'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import './styles/index.css'

interface AppState {
  stations: StationView[]
  favorites: StationView[]
  player: PlayerView | null
  error: string | null
  loading: boolean
}

function App() {
  const [collapsed, setCollapsed] = useState<{ stations: boolean; favorites: boolean }>({
    stations: false,
    favorites: true,
  })

  const [state, setState] = useState<AppState>({
    stations: [],
    favorites: [],
    player: null,
    error: null,
    loading: true,
  })

  const browsePoolRef = useRef<StationView[]>([])
  // Volume to return to when a muted stream is unmuted: muting to 0 and then
  // restoring to 0 would need the slider dragged back up by hand.
  const volumeBeforeMute = useRef(0.8)
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // A request overtaken by a newer one is not a failure: the newer request is
  // already answering, so this one must not touch the list or the spinner.
  const isSuperseded = (e: unknown) => String(e).includes('已被新的操作取代')

  const errorTimer = useRef<number | undefined>(undefined)

  const showError = useCallback((message: string) => {
    // One timer for the notice: a second failure has to restart it, or the
    // first timer would clear the newer message before it could be read.
    window.clearTimeout(errorTimer.current)
    setState(s => ({ ...s, error: message }))
    errorTimer.current = window.setTimeout(() => setState(s => ({ ...s, error: null })), 5000)
  }, [])

  const loadTopStations = useCallback(async () => {
    setState(s => ({ ...s, loading: true, error: null }))
    try {
      const stations = await invoke<StationView[]>('top_stations')
      browsePoolRef.current = stations
      setState(s => ({ ...s, stations, loading: false }))
    } catch (e) {
      if (isSuperseded(e)) return
      showError(`加载失败: ${e}`)
      setState(s => ({ ...s, loading: false }))
    }
  }, [showError])

  const searchStations = useCallback(async (term: string) => {
    setState(s => ({ ...s, loading: true, error: null }))
    try {
      const stations = await invoke<StationView[]>('search_stations', { term })
      browsePoolRef.current = stations
      setState(s => ({ ...s, stations, loading: false }))
    } catch (e) {
      if (isSuperseded(e)) return
      showError(`搜索失败: ${e}`)
      setState(s => ({ ...s, loading: false }))
    }
  }, [showError])

  const loadFavorites = useCallback(async (filter = '') => {
    try {
      const stations = await invoke<StationView[]>('get_favorites', { filter })
      setState(s => ({ ...s, favorites: stations }))
    } catch (e) {
      showError(`加载收藏失败: ${e}`)
    }
  }, [showError])

  // Re-reading both lists after a star was clicked replaced the whole array,
  // which remounted every tile and reloaded its artwork -- the list visibly
  // flashed. The store already decided the new state, so the one row it belongs
  // to is patched in place instead.
  const toggleFavorite = useCallback(async (station: StationView) => {
    try {
      const favorite = await invoke<boolean>('toggle_favorite', { station })
      setState(s => {
        const patch = (list: StationView[]) =>
          list.map(item => (item.key === station.key ? { ...item, favorite } : item))
        const favorites = patch(s.favorites)
        return {
          ...s,
          // The player bar shows a star for what is playing, so it has to learn
          // about the change too or it disagrees with the list.
          player:
            s.player?.station?.key === station.key && s.player.station
              ? { ...s.player, station: { ...s.player.station, favorite } }
              : s.player,
          stations: patch(s.stations),
          // A newly saved station is not in the favourites list yet, so it is
          // appended; an unfavourited one has to be dropped, or it stays on
          // screen with the store no longer holding it.
          favorites: favorite
            ? (favorites.some(f => f.key === station.key)
                ? favorites
                : [...favorites, { ...station, favorite: true }])
            : favorites.filter(f => f.key !== station.key),
        }
      })
    } catch (e) {
      showError(`操作失败: ${e}`)
    }
  }, [showError])

  // Clearing empties the store outright and there is no undo, so it is asked for
  // twice. The second click is only counted while the prompt is still up, so
  // leaving it on screen cannot leave a trap where the next click wipes the list.
  const [confirmingClear, setConfirmingClear] = useState(false)
  const confirmTimer = useRef<number | undefined>(undefined)

  const askClear = useCallback(() => {
    setConfirmingClear(true)
    window.clearTimeout(confirmTimer.current)
    confirmTimer.current = window.setTimeout(() => setConfirmingClear(false), 4000)
  }, [])

  const clearFavorites = useCallback(async () => {
    setConfirmingClear(false)
    window.clearTimeout(confirmTimer.current)
    try {
      await invoke('clear_favorites')
      setState(s => ({ ...s, favorites: [] }))
    } catch (e) {
      showError(`清空失败: ${e}`)
    }
  }, [showError])

  // Filled in below, once refreshPlayerState exists. Going through a ref keeps
  // `act` from having to be declared after the callback it calls, which is an
  // ordering trap: a useCallback referenced before its declaration is a
  // ReferenceError during render and takes the whole view down.
  const refreshRef = useRef<() => Promise<unknown>>(async () => null)

  // Runs a control and reflects the result at once, instead of leaving the UI to
  // wait for the next poll to notice.
  const act = useCallback(async (run: () => Promise<unknown>, failure: string) => {
    try {
      await run()
    } catch (e) {
      showError(`${failure}: ${e}`)
    }
    await refreshRef.current()
  }, [showError])

  const playStation = useCallback(
    (station: StationView) => act(() => invoke('play_station', { station }), '播放失败'),
    [act],
  )

  const playPause = useCallback(
    () => act(() => invoke('play_pause'), '操作失败'),
    [act],
  )

  const stopPlayback = useCallback(
    () => act(() => invoke('stop_playback'), '停止失败'),
    [act],
  )

  const playRandom = useCallback(
    (list: 'browse' | 'favorites') => act(() => invoke('play_random', { list }), '随机播放失败'),
    [act],
  )

  // The backend answers with the value it actually applied, which is what the
  // slider shows: the raw request is clamped to 0..1 there, and echoing that
  // back keeps the thumb from being dragged back by the next poll.
  const setVolume = useCallback(async (volume: number) => {
    try {
      const applied = await invoke<number>('set_volume', { v: volume })
      if (applied > 0) {
        volumeBeforeMute.current = applied
      }
      setState(s => (s.player ? { ...s, player: { ...s.player, volume: applied } } : s))
    } catch (e) {
      showError(`音量调整失败: ${e}`)
    }
  }, [showError])

  const toggleMute = useCallback(async () => {
    const current = state.player?.volume ?? 0
    const next = current > 0 ? 0 : volumeBeforeMute.current || 0.8
    await setVolume(next)
  }, [state.player?.volume, setVolume])

  // Returns what the player reported, so a caller can decide what to do next
  // from the answer it just got rather than from a value captured earlier.
  const refreshPlayerState = useCallback(async (): Promise<PlayerView | null> => {
    try {
      const player = await invoke<PlayerView>('get_state')
      setState(s => ({ ...s, player }))
      return player
    } catch (e) {
      console.error('Get state failed:', e)
      return null
    }
  }, [])

  useEffect(() => {
    refreshRef.current = refreshPlayerState
  }, [refreshPlayerState])

  useEffect(() => {
    loadTopStations()
    loadFavorites()
    refreshPlayerState()

    // The poll exists to keep the ICY title and the playing marker fresh. With
    // nothing playing there is nothing to keep fresh, so it stands down.
    let timer: number | undefined
    const tick = async () => {
      const player = await refreshPlayerState()
      timer = window.setTimeout(tick, player?.playing ? 1000 : 4000)
    }
    timer = window.setTimeout(tick, 1000)
    return () => window.clearTimeout(timer)
  }, [loadTopStations, loadFavorites, refreshPlayerState])

  useEffect(() => {
    // listen() resolves asynchronously, so a cleanup that runs first would find
    // nothing to cancel and leave the listener attached for good. The flag tells
    // a late resolution to cancel itself instead.
    let disposed = false
    let unlisten: UnlistenFn | undefined
    listen<PlayerView>('player', event => {
      setState(s => ({ ...s, player: event.payload }))
    })
      .then(fn => {
        if (disposed) fn()
        else {
          unlisten = fn
          unlistenRef.current = fn
        }
      })
      .catch(err => showError(`播放状态监听失败：${err}`))
    return () => {
      disposed = true
      unlisten?.()
      unlistenRef.current = null
    }
  }, [showError])

  // The search boxes are deliberately NOT controlled. While an IME composes a
  // word it owns the field's value, and a controlled input would write React's
  // stale value back over the pinyin buffer on every keystroke, which WebKitGTK
  // resolves by tearing down the view -- a black window. Reading the value from
  // the event target keeps React out of the field entirely.
  const searchTimer = useRef<number | undefined>(undefined)
  const favTimer = useRef<number | undefined>(undefined)

  const handleSearch = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const term = e.target.value
    window.clearTimeout(searchTimer.current)
    searchTimer.current = window.setTimeout(() => searchStations(term), 300)
  }, [searchStations])

  // Enter searches straight away rather than waiting out the debounce, and drops
  // the pending timer so the same term is not searched a second time.
  const handleSearchKey = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return
    e.preventDefault()
    window.clearTimeout(searchTimer.current)
    searchStations(e.currentTarget.value)
  }, [searchStations])

  const handleFavFilter = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const filter = e.target.value
    window.clearTimeout(favTimer.current)
    favTimer.current = window.setTimeout(() => loadFavorites(filter), 250)
  }, [loadFavorites])

  const handleFavKey = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return
    e.preventDefault()
    window.clearTimeout(favTimer.current)
    loadFavorites(e.currentTarget.value)
  }, [loadFavorites])

  useEffect(() => () => {
    window.clearTimeout(searchTimer.current)
    window.clearTimeout(favTimer.current)
    window.clearTimeout(confirmTimer.current)
    window.clearTimeout(errorTimer.current)
  }, [])


  const handleClose = useCallback(async () => {
    try {
      const win = await WebviewWindow.getByLabel('main')
      if (!win) throw new Error('找不到窗口')
      await win.close()
    } catch (e) {
      showError(`关闭失败: ${e}`)
    }
  }, [showError])

  const handleMinimize = useCallback(async () => {
    try {
      const win = await WebviewWindow.getByLabel('main')
      if (!win) throw new Error('找不到窗口')
      await win.minimize()
    } catch (e) {
      showError(`最小化失败: ${e}`)
    }
  }, [showError])

  return (
    <div className="app">
      <Titlebar onClose={handleClose} onMinimize={handleMinimize} />
      <div className="main">
        <div className="content">
          <main className="content-body">
            <section className="panel">
                <div className="panel-head-row">
                  <button
                    className="panel-head"
                    onClick={() => setCollapsed(c => ({ ...c, stations: !c.stations }))}
                    aria-expanded={!collapsed.stations}
                  >
                    <svg className={`panel-caret ${collapsed.stations ? 'closed' : ''}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M6 9l6 6 6-6" />
                    </svg>
                    <span className="panel-title">电台</span>
                    <span className="panel-count">{state.stations.length}</span>
                  </button>
                  <div className="panel-controls">
                    <div className="search-box">
                      <svg className="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <circle cx="11" cy="11" r="8" />
                        <path d="M21 21l-4.35-4.35" />
                      </svg>
                      <input
                        type="search"
                        className="search-input"
                        placeholder="搜索电台名…"
                        defaultValue=""
                        onChange={handleSearch}
                        onKeyDown={handleSearchKey}
                      />
                    </div>
                  </div>
                </div>
              <div className="panel-body-inner" hidden={collapsed.stations}>

                  {state.loading ? (
                    <div className="loading"><div className="spinner" /></div>
                  ) : state.stations.length === 0 ? (
                    <p className="panel-empty">没有找到电台，试试其他关键词或检查网络连接</p>
                  ) : (
                    <StationGrid
                      stations={state.stations}
                      playingKey={state.player?.playingKey ?? null}
                      onPlay={playStation}
                      onToggleFavorite={toggleFavorite}
                    />
                  )}
                </div>
            </section>

            <section className="panel">
              <div className="panel-head-row">
                <button
                  className="panel-head"
                  onClick={() => setCollapsed(c => ({ ...c, favorites: !c.favorites }))}
                  aria-expanded={!collapsed.favorites}
                >
                  <svg className={`panel-caret ${collapsed.favorites ? 'closed' : ''}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M6 9l6 6 6-6" />
                  </svg>
                  <span className="panel-title">收藏</span>
                  <span className="panel-count">{state.favorites.length}</span>
                </button>
                <div className="panel-controls">
                  <div className="search-box">
                    <svg className="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <circle cx="11" cy="11" r="8" />
                      <path d="M21 21l-4.35-4.35" />
                    </svg>
                    <input
                      type="search"
                      className="search-input"
                      placeholder="筛选收藏…"
                      defaultValue=""
                      onChange={handleFavFilter}
                      onKeyDown={handleFavKey}
                    />
                  </div>
                  {state.favorites.length > 0 && (confirmingClear ? (
                      <button
                        className="btn btn-danger"
                        onClick={clearFavorites}
                        onBlur={() => setConfirmingClear(false)}
                        title="再次点击以确认清空"
                      >
                        确认清空？
                      </button>
                    ) : (
                      <button className="btn" onClick={askClear} title="清空收藏">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="15" height="15">
                          <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" />
                        </svg>
                      </button>
                  ))}
                </div>
              </div>
              <div className="panel-body-inner" hidden={collapsed.favorites}>

                  {state.favorites.length === 0 ? (
                    <p className="panel-empty">还没有收藏，点击电台卡片右上角的星标即可添加</p>
                  ) : (
                    <StationGrid
                      stations={state.favorites}
                      playingKey={state.player?.playingKey ?? null}
                      onPlay={playStation}
                      onToggleFavorite={toggleFavorite}
                    />
                  )}
                </div>
            </section>
          </main>
        </div>
      </div>
      <PlayerBar
        player={state.player}
        onPlayPause={playPause}
        onStop={stopPlayback}
        onPlayRandomBrowse={() => playRandom('browse')}
        onPlayRandomFavorites={() => playRandom('favorites')}
        onVolumeChange={setVolume}
        onToggleFavorite={() => {
          const st = state.player?.station
          if (st) toggleFavorite(st)
        }}
        isMuted={(state.player?.volume ?? 0) <= 0}
        onToggleMute={toggleMute}
      />
      {state.error && <Toast message={state.error} onClose={() => setState(s => ({ ...s, error: null }))} />}
    </div>
  )
}

export default App