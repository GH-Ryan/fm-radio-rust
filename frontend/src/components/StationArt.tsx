import { useEffect, useRef, useState } from 'react'
import { StationView } from '../types'

/**
 * A stable colour per station, so a tile that falls back to a letter or a flag
 * still looks like it belongs to that station rather than being a grey gap.
 */
function artColor(seed: string): string {
  let hash = 0
  for (const ch of seed) hash = (hash * 31 + ch.codePointAt(0)!) >>> 0
  return `hsl(${hash % 360} 34% 34%)`
}

/** ISO 3166-1 alpha-2 to a flag emoji; anything else has no flag. */
function flagOf(code: string): string {
  const c = (code || '').trim()
  if (c.length !== 2) return ''
  return String.fromCodePoint(
    ...[...c.toUpperCase()].map(ch => 0x1f1e6 + ch.charCodeAt(0) - 65),
  )
}

/**
 * Station metadata comes from public catalogue records, so the logo URL is
 * treated as untrusted: only http and https are fetched. Any other scheme --
 * including an image-bearing data URL -- would turn one record into an
 * unexpectedly large embedded download.
 */
export function artworkSource(station: StationView): string {
  const raw = typeof station.favicon === 'string' ? station.favicon.trim() : ''
  if (!raw) return ''
  const candidate = raw.startsWith('//') ? `https:${raw}` : raw
  try {
    const url = new URL(candidate)
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return ''
    return url.href
  } catch {
    return ''
  }
}

/**
 * The cover is drawn in layers: the station's letter or flag goes down first and
 * the logo replaces it only once it has actually loaded. A dead URL -- and many
 * of the catalogue's are -- therefore leaves a legible tile rather than a broken
 * image icon.
 */
export function StationArt({ station, className }: { station: StationView; className: string }) {
  const [loaded, setLoaded] = useState(false)
  const imgRef = useRef<HTMLImageElement>(null)
  const source = artworkSource(station)

  // A new station in the same tile must not inherit the previous one's state.
  useEffect(() => { setLoaded(false) }, [source])

  // React's synthetic onLoad does not fire reliably for a lazily loaded image,
  // so the listener is attached to the element itself.
  useEffect(() => {
    const img = imgRef.current
    if (!img) return
    const ok = () => setLoaded(true)
    const bad = () => setLoaded(false)
    img.addEventListener('load', ok)
    img.addEventListener('error', bad)
    // An image restored from cache can finish before React attaches the handler.
    if (img.complete && img.naturalWidth > 0) setLoaded(true)
    return () => {
      img.removeEventListener('load', ok)
      img.removeEventListener('error', bad)
    }
  }, [source])

  const flag = flagOf(station.countryCode)
  const letter = (station.displayName || '?').trim().charAt(0).toUpperCase()

  return (
    <div className={className} style={{ background: artColor(station.key) }}>
      {!loaded && (
        <span className="station-art-fallback">{flag || letter}</span>
      )}
      {source && (
        <img
          ref={imgRef}
          key={source}
          src={source}
          alt=""
          decoding="async"
          // Kept in the render tree and only made transparent: a `hidden` img is
          // not laid out, so the browser never fetches it and onLoad never runs,
          // which would leave every tile stuck on its fallback.
          className={loaded ? 'art-loaded' : 'art-pending'}
          onLoad={() => setLoaded(true)}
        />
      )}
    </div>
  )
}
