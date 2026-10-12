import { StationView } from '../types'
import { MetaIcons } from './MetaIcons'
import { StationArt } from './StationArt'
import { useI18n } from '../i18n/useI18n'

interface StationGridProps {
  stations: StationView[]
  playingKey: string | null
  onPlay: (station: StationView) => void
  onToggleFavorite: (station: StationView) => void
}

export function StationGrid({ stations, playingKey, onPlay, onToggleFavorite }: StationGridProps) {
  const { t } = useI18n()

  return (
    <div className="station-grid" role="list">
      {stations.map((station) => (
        <article
          key={station.key}
          className={`station-card ${playingKey === station.key ? 'playing' : ''}`}
          title={station.meta}
          role="listitem"
          onClick={() => onPlay(station)}
        >
          <div className="station-card-header">
            <StationArt station={station} className="station-card-image" />
            <div className="station-card-play">
              <button
                className="station-card-play-btn"
                onClick={(e) => { e.stopPropagation(); onPlay(station); }}
                aria-label={playingKey === station.key ? t('cardPause') : t('cardPlay')}
              >
                {playingKey === station.key ? (
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
            </div>
            <button
              className={`station-card-favorite ${station.favorite ? 'active' : ''}`}
              onClick={(e) => { e.stopPropagation(); onToggleFavorite(station); }}
              aria-label={station.favorite ? t('unfavorite') : t('favorite')}
              aria-pressed={station.favorite}
            >
              <svg viewBox="0 0 24 24" fill={station.favorite ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth={2}>
                <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
              </svg>
            </button>
          </div>
          <div className="station-card-body">
            <h3 className="station-card-name" title={station.displayName}>{station.displayName}</h3>
            <MetaIcons station={station} />
          </div>
        </article>
      ))}
    </div>
  )
}