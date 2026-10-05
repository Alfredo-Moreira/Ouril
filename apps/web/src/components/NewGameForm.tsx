/**
 * Choosing and starting a new game: difficulty, who starts, then Start. Used on the home page
 * and on the game screen when no game is in progress, so starting is always one screen.
 */
import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router';

import { LEVELS, type Level } from '../engine/types';
import { PLAYABLE_VARIANTS, type PlayableVariant, type Starter } from '../game/session';
import { ArrowIcon } from './Icons';

export interface NewGameRequest {
  level: Level;
  starter: Starter;
  /** Variant ID; absent = standard Ouril. */
  variant?: PlayableVariant;
}

const STARTERS: Starter[] = ['me', 'ai', 'random'];

export function NewGameForm({
  title,
  intro,
  replaceWarning = false,
  headingLevel = 2,
}: {
  title: string;
  intro?: string;
  /** A game is in progress and starting will replace it. */
  replaceWarning?: boolean;
  headingLevel?: 1 | 2;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [level, setLevel] = useState<Level>('easy');
  const [starter, setStarter] = useState<Starter>('random');
  const [variant, setVariant] = useState<PlayableVariant>('cv.standard');
  const Heading = headingLevel === 1 ? 'h1' : 'h2';

  const start = (e: FormEvent) => {
    e.preventDefault();
    const request: NewGameRequest = { level, starter, variant };
    navigate('/play', { state: { newGame: request } });
  };

  return (
    <form className="card new-game" onSubmit={start}>
      <Heading className="card__title">{title}</Heading>
      {intro && <p className="note">{intro}</p>}
      <fieldset>
        <legend>{t('home.level')}</legend>
        <div className="choice-row">
          {LEVELS.map((l) => (
            <label key={l} className="choice">
              <input
                type="radio"
                name="level"
                value={l}
                checked={level === l}
                onChange={() => setLevel(l)}
              />
              <span>{t(`game.level.${l}`)}</span>
            </label>
          ))}
        </div>
      </fieldset>
      <fieldset>
        <legend>{t('home.starter')}</legend>
        <div className="choice-row">
          {STARTERS.map((s) => (
            <label key={s} className="choice">
              <input
                type="radio"
                name="starter"
                value={s}
                checked={starter === s}
                onChange={() => setStarter(s)}
              />
              <span>{t(`home.starter.${s}`)}</span>
            </label>
          ))}
        </div>
      </fieldset>
      <fieldset>
        <legend>{t('home.variant')}</legend>
        <div className="choice-row choice-row--stack">
          {PLAYABLE_VARIANTS.map((v) => (
            <label key={v} className="choice">
              <input
                type="radio"
                name="variant"
                value={v}
                checked={variant === v}
                onChange={() => setVariant(v)}
              />
              <span>{t(`variant.${v}.short`)}</span>
            </label>
          ))}
        </div>
        <p className="note">{t(`variant.${variant}.about`)}</p>
      </fieldset>
      {replaceWarning && <p className="note">{t('home.replace_warning')}</p>}
      <button type="submit" className="button button--primary button--large">
        {t('home.start')}
        <ArrowIcon />
      </button>
    </form>
  );
}
