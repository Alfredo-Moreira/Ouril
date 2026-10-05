/**
 * Two recommended physical boards (classic and premium), each opening Amazon in a new tab.
 * Links come from `content/boards.ts`; empty ones are hidden in production.
 */
import { useTranslation } from 'react-i18next';

import { BOARD_KINDS, BOARDS } from '../content/boards';
import { ArrowIcon } from './Icons';

const SHOW_PLACEHOLDERS = import.meta.env.DEV;

export function BuyBoards({ id, headingLevel = 2 }: { id?: string; headingLevel?: 2 | 3 }) {
  const { t } = useTranslation();
  const kinds = BOARD_KINDS.filter((k) => BOARDS.links[k] || SHOW_PLACEHOLDERS);
  if (kinds.length === 0) return null;
  const Heading = headingLevel === 2 ? 'h2' : 'h3';
  const rel = BOARDS.affiliate ? 'sponsored noopener noreferrer' : 'noopener noreferrer';
  return (
    <section id={id} className="buy" aria-labelledby={`${id ?? 'buy'}-title`}>
      <Heading id={`${id ?? 'buy'}-title`} className="section-title buy__title">
        {t('shop.title')}
      </Heading>
      <p className="buy__body">{t('shop.body')}</p>
      <ul className="buy__list">
        {kinds.map((k) => {
          const url = BOARDS.links[k];
          return (
            <li key={k} className={`buy__item buy__item--${k}`}>
              <BoardArt premium={k === 'premium'} />
              <h3 className="buy__name">{t(`shop.${k}.title`)}</h3>
              <p className="buy__desc">{t(`shop.${k}.body`)}</p>
              {url ? (
                <a
                  className={`button ${k === 'premium' ? 'button--primary' : ''}`}
                  href={url}
                  target="_blank"
                  rel={rel}
                >
                  {t('shop.cta')}
                  <ArrowIcon />
                </a>
              ) : (
                <span className="button buy__placeholder">{t('shop.placeholder')}</span>
              )}
            </li>
          );
        })}
      </ul>
      {BOARDS.affiliate && <p className="note buy__disclosure">{t('shop.disclosure')}</p>}
    </section>
  );
}

/** A small drawing of a board (ours: Amazon's product photos can't be shown here). */
function BoardArt({ premium }: { premium: boolean }) {
  const pits = Array.from({ length: 6 }, (_, i) => i);
  return (
    <svg className="buy__art" viewBox="0 0 220 80" aria-hidden="true">
      <rect x="2" y="6" width="216" height="68" rx="34" className="buy__wood" />
      <ellipse cx="26" cy="40" rx="14" ry="26" className="buy__pit" />
      <ellipse cx="194" cy="40" rx="14" ry="26" className="buy__pit" />
      {pits.map((i) => (
        <g key={i}>
          <circle cx={55 + i * 22} cy="26" r="9" className="buy__pit" />
          <circle cx={55 + i * 22} cy="54" r="9" className="buy__pit" />
          <circle cx={53 + i * 22} cy="28" r="2.6" className="buy__seed" />
          <circle cx={57 + i * 22} cy="55" r="2.6" className="buy__seed" />
        </g>
      ))}
      {premium && <rect x="2" y="6" width="216" height="68" rx="34" className="buy__trim" />}
    </svg>
  );
}
