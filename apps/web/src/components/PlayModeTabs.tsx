import { useRef, type KeyboardEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { PLAY_MODES, type PlayMode } from '../game/playModes';

/**
 * Tabs to choose who to play (the computer or online) above a panel, as on the Game page and
 * the landing page's quick start. With only one mode, just the panel. Arrow keys move between
 * the tabs.
 */
export function PlayModeTabs({
  mode,
  onSelect,
  idPrefix,
  children,
}: {
  mode: PlayMode;
  onSelect: (mode: PlayMode) => void;
  /** Keeps element IDs unique when there's more than one set of tabs. */
  idPrefix: string;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const tabbed = PLAY_MODES.length > 1;
  const panelId = `${idPrefix}-panel`;

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const n = PLAY_MODES.length;
    const next = PLAY_MODES[(PLAY_MODES.indexOf(mode) + step + n) % n]!;
    onSelect(next);
    tabs.current[PLAY_MODES.indexOf(next)]?.focus();
  };

  return (
    <>
      {tabbed && (
        <div
          className="rules__tabs choice-row"
          role="tablist"
          aria-label={t('play.mode')}
          onKeyDown={onKeyDown}
        >
          {PLAY_MODES.map((m, i) => (
            <button
              key={m}
              ref={(el) => {
                tabs.current[i] = el;
              }}
              type="button"
              role="tab"
              id={`${idPrefix}-tab-${m}`}
              aria-selected={m === mode}
              aria-controls={panelId}
              tabIndex={m === mode ? 0 : -1}
              className="choice rules__tab"
              onClick={() => onSelect(m)}
            >
              {t(`play.mode.${m}`)}
            </button>
          ))}
        </div>
      )}
      <div
        id={panelId}
        role={tabbed ? 'tabpanel' : undefined}
        aria-labelledby={tabbed ? `${idPrefix}-tab-${mode}` : undefined}
        className="game-hub__panel"
      >
        {children}
      </div>
    </>
  );
}
