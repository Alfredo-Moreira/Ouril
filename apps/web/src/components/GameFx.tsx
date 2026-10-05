/**
 * Game feedback: the score pop when seeds are captured, the end-of-game count-up and the win
 * celebration. Motion rules (docs/architecture/web-design.md): feedback stays short and
 * readable; only a win celebrates (a loss is calm); reduced motion gets the final state at once.
 */
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  );
}

/**
 * A score that pops when it goes up, with a "+N" rising from it. The number updates at once
 * (the value is never behind the game); the pop and the "+N" are decoration.
 */
export function ScoreValue({ value }: { value: number }) {
  const { t } = useTranslation();
  const [prev, setPrev] = useState(value);
  const [gain, setGain] = useState<{ n: number; id: number } | null>(null);
  if (value !== prev) {
    setPrev(value);
    setGain(value > prev ? { n: value - prev, id: (gain?.id ?? 0) + 1 } : null);
  }
  return (
    <span className="score__value">
      <span key={gain?.id ?? 0} className={gain ? 'score__num score__num--pop' : 'score__num'}>
        {value}
      </span>
      {gain && (
        <span key={`gain-${gain.id}`} className="score__gain" aria-hidden="true">
          {t('game.score.gained', { count: gain.n })}
        </span>
      )}
    </span>
  );
}

// --- Win celebration ----------------------------------------------------------------------------

/**
 * A one-off burst in the colours of the Cape Verdean flag, led by ten yellow stars (the flag's
 * ten islands), with a few grey ouris among them. Plays once, for about two seconds, behind the
 * result dialog; nothing with reduced motion.
 */
const COLORS = ['#003893', '#ffffff', '#cf2027', '#f7d116', '#0f6b75'];

export function Celebration() {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const root = ref.current;
    if (!root || prefersReducedMotion() || typeof root.animate !== 'function') return;
    const w = window.innerWidth;
    const h = window.innerHeight;
    const pieces: Animation[] = [];
    const rand = (a: number, b: number) => a + Math.random() * (b - a);
    const total = Math.min(70, Math.round(w / 16));
    for (let i = 0; i < total; i++) {
      const el = document.createElement('span');
      const star = i < 10;
      const seed = !star && i % 9 === 0;
      el.className = star
        ? 'confetti confetti--star'
        : seed
          ? 'confetti confetti--seed'
          : 'confetti';
      if (!star && !seed) el.style.background = COLORS[i % COLORS.length]!;
      root.appendChild(el);
      // Burst up and out from just above the dialog, then fall with gravity.
      const angle = rand(-Math.PI * 0.9, -Math.PI * 0.1);
      const power = rand(0.25, 0.55) * Math.min(w, 900);
      const dx = Math.cos(angle) * power;
      const up = Math.sin(angle) * power * 0.8;
      const fall = h * rand(0.55, 0.9);
      const spin = rand(-540, 540);
      const anim = el.animate(
        [
          // Burst: fast out, slowing at the top.
          {
            transform: 'translate(-50%, -50%) translate(0, 0) rotate(0deg) scale(0.4)',
            opacity: 1,
            easing: 'cubic-bezier(0.23, 1, 0.32, 1)',
          },
          {
            transform: `translate(-50%, -50%) translate(${dx * 0.8}px, ${up}px) rotate(${spin * 0.4}deg) scale(1)`,
            opacity: 1,
            offset: 0.3,
            // Fall: accelerating, like gravity (physics here, not UI easing).
            easing: 'cubic-bezier(0.55, 0, 1, 0.45)',
          },
          {
            transform: `translate(-50%, -50%) translate(${dx}px, ${up + fall}px) rotate(${spin}deg) scale(0.9)`,
            opacity: 0,
          },
        ],
        {
          duration: rand(1600, 2400),
          delay: star ? i * 40 : rand(0, 180),
          easing: 'linear',
          fill: 'both',
        },
      );
      pieces.push(anim);
    }
    return () => {
      pieces.forEach((a) => a.cancel());
      root.replaceChildren();
    };
  }, []);
  return <div ref={ref} className="celebration" aria-hidden="true" />;
}
