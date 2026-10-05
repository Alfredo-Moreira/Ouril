import { useRef, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router';

import { RulesView } from '../components/RulesView';
import { PLAYABLE_VARIANTS, type PlayableVariant } from '../game/session';

/**
 * Rules reference: one tab per playable variant (`?variant=<id>`, so a tab can be linked),
 * each a compact list of collapsible sections. Text: `content/rules.ts` + `rules.*` strings.
 */
export function Rules() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const requested = params.get('variant');
  const variant: PlayableVariant =
    PLAYABLE_VARIANTS.find((v) => v === requested) ?? PLAYABLE_VARIANTS[0];
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);

  const select = (v: PlayableVariant) =>
    setParams(v === PLAYABLE_VARIANTS[0] ? {} : { variant: v }, { replace: true });

  // Arrow keys move between tabs (WAI-ARIA tabs pattern, automatic activation).
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const i = PLAYABLE_VARIANTS.indexOf(variant);
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const next =
      PLAYABLE_VARIANTS[(i + step + PLAYABLE_VARIANTS.length) % PLAYABLE_VARIANTS.length]!;
    select(next);
    tabs.current[PLAYABLE_VARIANTS.indexOf(next)]?.focus();
  };

  return (
    <article className="rules prose">
      <h1>{t('rules.title')}</h1>
      <p>
        <Link to="/tutorial">{t('rules.try_tutorial')}</Link>
      </p>
      <div
        className="rules__tabs choice-row"
        role="tablist"
        aria-label={t('home.variant')}
        onKeyDown={onKeyDown}
      >
        {PLAYABLE_VARIANTS.map((v, i) => (
          <button
            key={v}
            ref={(el) => {
              tabs.current[i] = el;
            }}
            type="button"
            role="tab"
            id={`rules-tab-${v}`}
            aria-selected={v === variant}
            aria-controls="rules-panel"
            tabIndex={v === variant ? 0 : -1}
            className="choice rules__tab"
            onClick={() => select(v)}
          >
            {t(`variant.${v}.short`)}
          </button>
        ))}
      </div>
      <div
        id="rules-panel"
        role="tabpanel"
        aria-labelledby={`rules-tab-${variant}`}
        className="rules__panel"
      >
        <RulesView key={variant} variant={variant} onShowVariant={select} />
      </div>
    </article>
  );
}
