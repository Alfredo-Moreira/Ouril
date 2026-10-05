/**
 * One variant's rules as a compact list of collapsible sections (native <details>, so it's
 * keyboard- and screen-reader-friendly without extra code). Used by the Rules page and the
 * in-game rules dialog.
 */
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { VARIANT_RULES } from '../content/rules';
import type { PlayableVariant } from '../game/session';

export function RulesView({
  variant,
  onShowVariant,
  headingLevel = 2,
}: {
  variant: PlayableVariant;
  /** Lets a "differences only" variant link to the rules it's based on. */
  onShowVariant?: (v: PlayableVariant) => void;
  headingLevel?: 2 | 3;
}) {
  const { t } = useTranslation();
  const rules = VARIANT_RULES[variant];
  // All sections start closed except the first; "Expand all" opens every one at once.
  const [open, setOpen] = useState<Record<string, boolean>>(() => ({
    [rules.sections[0]!.id]: true,
  }));
  const allOpen = rules.sections.every((s) => open[s.id]);
  const Heading = headingLevel === 2 ? 'h2' : 'h3';

  return (
    <div className="rules-view">
      <p className="rules-view__intro">{t(rules.intro)}</p>
      {rules.basedOn && (
        <p className="note">
          {t('rules.based_on', { name: t(`variant.${rules.basedOn}.short`) })}{' '}
          {onShowVariant && (
            <button
              type="button"
              className="link-button"
              onClick={() => onShowVariant(rules.basedOn!)}
            >
              {t('rules.show_variant', { name: t(`variant.${rules.basedOn}.short`) })}
            </button>
          )}
        </p>
      )}
      <div className="rules-view__tools">
        <button
          type="button"
          className="link-button"
          onClick={() =>
            setOpen(allOpen ? {} : Object.fromEntries(rules.sections.map((s) => [s.id, true])))
          }
        >
          {allOpen ? t('rules.collapse_all') : t('rules.expand_all')}
        </button>
      </div>
      {rules.sections.map(({ id, paragraphs }) => (
        <details
          key={id}
          className="rule"
          open={!!open[id]}
          onToggle={(e) => {
            const isOpen = (e.currentTarget as HTMLDetailsElement).open;
            setOpen((o) => (o[id] === isOpen ? o : { ...o, [id]: isOpen }));
          }}
        >
          <summary className="rule__summary">
            <Heading className="rule__title">{t(`rules.${id}.title`)}</Heading>
          </summary>
          <div className="rule__body">
            {Array.from({ length: paragraphs }, (_, i) => (
              <p key={i}>{t(`rules.${id}.${i + 1}`)}</p>
            ))}
          </div>
        </details>
      ))}
    </div>
  );
}
