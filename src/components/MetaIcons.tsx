import { StationView } from '../types'

/** Bitrate expressed as 0-4 bars, so a weak stream is visible at a glance. */
/** ISO 3166-1 alpha-2 to a flag emoji; a code of any other shape has no flag. */
function flagOf(code: string): string {
  const c = (code || '').trim()
  if (!/^[a-zA-Z]{2}$/.test(c)) return ''
  return String.fromCodePoint(
    ...[...c.toUpperCase()].map(ch => 0x1f1e6 + ch.charCodeAt(0) - 65),
  )
}

function barsFor(kbps: number): number {
  if (kbps <= 0) return 0
  if (kbps < 64) return 1
  if (kbps < 128) return 2
  if (kbps < 256) return 3
  return 4
}

function codecTone(codec: string): string {
  const c = codec.toUpperCase()
  if (c.startsWith('MP3')) return 'mp3'
  if (c.startsWith('AAC')) return 'aac'
  if (c.startsWith('FLAC')) return 'flac'
  if (c.startsWith('OGG') || c.startsWith('VORBIS')) return 'ogg'
  if (c.startsWith('OPUS')) return 'opus'
  if (c.startsWith('WAV')) return 'wav'
  return 'unknown'
}

export function MetaIcons({ station }: { station: StationView }) {
  const flag = flagOf(station.countryCode)
  const bars = barsFor(station.bitrate)
  const tone = codecTone(station.codec)

  return (
    <div className="meta-icons" title={station.meta}>
      {flag && <span className="meta-flag">{flag}</span>}

      {bars > 0 && (
        <span className="meta-signal" aria-label={`${station.bitrate} kbps`}>
          {[1, 2, 3, 4].map(n => (
            <i key={n} className={n <= bars ? 'on' : ''} style={{ height: 3 + n * 2 }} />
          ))}
        </span>
      )}

      <span className={`meta-codec tone-${tone}`}>
        {/* codecKnown rather than a comparison against the label: the label for
            an unnamed codec is a sentence, and matching on it breaks as soon as
            the backend is localised or the wording is edited. */}
        {station.codecKnown ? station.codec : '?'}
      </span>
    </div>
  )
}
