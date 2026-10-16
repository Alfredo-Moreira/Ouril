import { useTranslation } from 'react-i18next';

import { hasMusic, skipTrack } from '../audio/music';
import { useMusicPrefs } from '../audio/musicPrefs';
import { MUSIC_PLAYLIST_URL } from '../content/music';
import { trackEvent } from '../telemetry';
import { MusicIcon, MusicOffIcon, PlaylistIcon, SkipIcon } from './Icons';

/**
 * The game screens' music buttons, joined as one group: play or pause, next track (while
 * playing), and a link to the music's YouTube playlist, which opens in a new tab. Nothing
 * when there's no music.
 */
export function MusicControls() {
  const { t } = useTranslation();
  const [music, setMusicPrefs] = useMusicPrefs();
  if (!hasMusic()) return null;
  const playlist = t('game.music.playlist');
  return (
    <div className="button-group" role="group" aria-label={t('game.music.controls')}>
      <button
        type="button"
        className="icon-button"
        onClick={() => void setMusicPrefs({ on: !music.on })}
        aria-label={t(music.on ? 'game.music.pause' : 'game.music.play')}
        aria-pressed={music.on}
      >
        {music.on ? <MusicIcon /> : <MusicOffIcon />}
      </button>
      {music.on && (
        <button
          type="button"
          className="icon-button"
          onClick={skipTrack}
          aria-label={t('game.music.skip')}
        >
          <SkipIcon />
        </button>
      )}
      <a
        className="icon-button"
        href={MUSIC_PLAYLIST_URL}
        target="_blank"
        rel="noopener noreferrer"
        title={playlist}
        aria-label={playlist}
        onClick={() => trackEvent('music_playlist_open')}
      >
        <PlaylistIcon />
      </a>
    </div>
  );
}
