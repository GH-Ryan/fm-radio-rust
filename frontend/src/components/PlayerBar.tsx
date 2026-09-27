import { PlayerView } from '../types'
import { artworkSource } from './StationArt'

interface PlayerBarProps {
  player: PlayerView | null
  onPlayPause: () => void
  onStop: () => void
  onPlayRandomBrowse: () => void
  onPlayRandomFavorites: () => void
  onVolumeChange: (volume: number) => void
  isMuted: boolean
  onToggleMute: () => void
  onToggleFavorite: () => void
}

export function PlayerBar({ player, onPlayPause, onStop, onVolumeChange, isMuted, onToggleMute, onToggleFavorite, onPlayRandomBrowse, onPlayRandomFavorites }: PlayerBarProps) {
  if (!player) return null

  const isPlaying = player.playing && !player.paused
  const percent = Math.round(player.volume * 100)
  // Nothing to start, and the button would only report an error.
  const canStart = isPlaying || Boolean(player.station)

  return (
    <>
    {player.error && (
      <div className="np-error" role="status" aria-live="polite">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
          <circle cx="12" cy="12" r="9" />
          <path d="M12 8v5M12 16.5v.5" />
        </svg>
        <span>{player.error}</span>
      </div>
    )}
    <footer className="player-bar" role="region" aria-label="播放器控制">
      <div className="player-info">
        {player.station && artworkSource(player.station) ? (
          <img className="player-artwork" src={artworkSource(player.station)} alt="" />
        ) : (
          <div className="player-artwork" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#1a1c21' }}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" style={{ width: 24, height: 24, opacity: 0.5 }}>
              <path d="M12 2a10 10 0 0 1 10 10c0 1.6-.6 3.1-1.7 4.2" />
              <path d="M19 21l-7-7-7 7V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
            </svg>
          </div>
        )}
        <div className="player-details">
          <div className="player-title">
            {player.title || player.name || '未知电台'}
          </div>
          <div className="player-station">
            {player.station?.displayName || player.name}
            {player.backend && ` · ${player.backend}`}
          </div>
        </div>
      </div>

      <div className="player-controls">
        <button
          className="player-btn"
          onClick={onPlayRandomBrowse}
          title="随机播放（电台列表）"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="19" height="19">
            <rect x="3" y="3" width="18" height="18" rx="4" />
            <circle cx="8.5" cy="8.5" r="1.4" fill="currentColor" stroke="none" />
            <circle cx="15.5" cy="15.5" r="1.4" fill="currentColor" stroke="none" />
            <circle cx="12" cy="12" r="1.4" fill="currentColor" stroke="none" />
          </svg>
        </button>
        <button
          className="player-btn"
          onClick={onPlayRandomFavorites}
          title="随机播放（收藏列表）"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="19" height="19">
            <path d="M12 20.5l-1.3-1.2C6 15 3 12.3 3 8.9 3 6.1 5.2 4 7.9 4c1.5 0 3 .7 3.9 1.8C12.8 4.7 14.2 4 15.8 4 18.5 4 20.7 6.1 20.7 8.9c0 3.4-3 6.1-7.7 10.4L12 20.5z" />
            <path d="M16.5 2.5l1 2.2 2.2 1-2.2 1-1 2.2-1-2.2-2.2-1 2.2-1 1-2.2z" fill="currentColor" stroke="none" />
          </svg>
        </button>
        <button
          className="player-btn play-pause"
          onClick={onPlayPause}
          disabled={!canStart}
          title={isPlaying ? '暂停' : '播放'}
          aria-label={isPlaying ? '暂停' : '播放'}
        >
          {isPlaying ? (
            <svg viewBox="0 0 24 24" fill="currentColor" width="20" height="20">
              <rect x="6" y="4" width="4" height="16" />
              <rect x="14" y="4" width="4" height="16" />
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" fill="currentColor" width="20" height="20">
              <path d="M8 5v14l11-7z" />
            </svg>
          )}
        </button>
        <button
          className="player-btn"
          onClick={onStop}
          title="停止"
          disabled={!player.playing}
        >
          <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
            <rect x="6" y="4" width="12" height="16" />
          </svg>
        </button>
        <button
          className="player-btn"
          onClick={onToggleFavorite}
          title={player.station?.favorite ? '取消收藏' : '收藏'}
          aria-label={player.station?.favorite ? '取消收藏' : '收藏'}
          aria-pressed={player.station?.favorite ?? false}
          disabled={!player.station}
          style={{ color: player.station?.favorite ? 'var(--star)' : undefined }}
        >
          <svg viewBox="0 0 24 24" fill={player.station?.favorite ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="2" width="20" height="20">
            <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
          </svg>
        </button>
      </div>

      <div className="player-volume">
        <button
          className="player-btn"
          onClick={onToggleMute}
          title={isMuted ? '取消静音' : '静音'}
          aria-label={isMuted ? '取消静音' : '静音'}
          aria-pressed={isMuted}
          style={{ width: 32, height: 32, flexShrink: 0, color: isMuted ? 'var(--accent)' : undefined }}
        >
          {isMuted ? (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="19" height="19">
              <path d="M11 5L6 9H2v6h4l5 4V5z" />
              <path d="M22 9l-6 6M16 9l6 6" />
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" width="19" height="19">
              <path d="M11 5L6 9H2v6h4l5 4V5z" />
              <path d="M15.5 8.5a5 5 0 010 7M18.5 5.5a9 9 0 010 13" />
            </svg>
          )}
        </button>
        <input
          type="range"
          className="volume-slider"
          min="0"
          max="1"
          step="0.01"
          value={player.volume}
          onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
          aria-label="音量"
        />
        <span className="volume-value">{percent}</span>
      </div>

    </footer>
    </>
  )
}