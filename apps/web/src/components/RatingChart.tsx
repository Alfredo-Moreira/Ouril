import { useId, useMemo, useState, type PointerEvent } from 'react';
import { useTranslation } from 'react-i18next';

import type { RatingPoint } from '../api/types';

const W = 320;
const H = 120;
const PAD = { top: 10, right: 12, bottom: 18, left: 36 };

/**
 * One pool's rating over time: a single 2 px line (no legend; the card's title names it),
 * recessive axis labels in muted ink, and a crosshair with a tooltip on hover. Every point is
 * also in a visually hidden table for screen readers.
 */
export function RatingChart({ points, title }: { points: RatingPoint[]; title: string }) {
  const { t, i18n } = useTranslation();
  const tableId = useId();
  const [hover, setHover] = useState<number | null>(null);
  const dates = useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium' }),
    [i18n.language],
  );

  const geo = useMemo(() => {
    const times = points.map((p) => Date.parse(p.at));
    const ratings = points.map((p) => p.rating);
    const t0 = Math.min(...times);
    const t1 = Math.max(...times);
    // Some headroom, so a flat line isn't glued to an edge.
    const lo = Math.min(...ratings) - 20;
    const hi = Math.max(...ratings) + 20;
    const x = (time: number) =>
      t1 === t0
        ? PAD.left + (W - PAD.left - PAD.right) / 2
        : PAD.left + ((time - t0) / (t1 - t0)) * (W - PAD.left - PAD.right);
    const y = (r: number) => PAD.top + (1 - (r - lo) / (hi - lo)) * (H - PAD.top - PAD.bottom);
    const xy = points.map((p, i) => ({ x: x(times[i]!), y: y(p.rating) }));
    return { xy, lo: Math.round(lo + 20), hi: Math.round(hi - 20) };
  }, [points]);

  if (points.length === 0) return null;
  const path = geo.xy.map((p, i) => `${i ? 'L' : 'M'}${p.x.toFixed(1)},${p.y.toFixed(1)}`).join('');
  const nearest = (e: PointerEvent<SVGSVGElement>) => {
    const box = e.currentTarget.getBoundingClientRect();
    const px = ((e.clientX - box.left) / box.width) * W;
    let best = 0;
    geo.xy.forEach((p, i) => {
      if (Math.abs(p.x - px) < Math.abs(geo.xy[best]!.x - px)) best = i;
    });
    setHover(best);
  };
  const h = hover === null ? null : { ...geo.xy[hover]!, point: points[hover]! };

  return (
    <figure className="rating-chart">
      <div className="rating-chart__plot">
        <svg
          viewBox={`0 0 ${W} ${H}`}
          role="img"
          aria-label={t('profile.chart.label', { pool: title })}
          aria-describedby={tableId}
          onPointerMove={nearest}
          onPointerLeave={() => setHover(null)}
        >
          <line
            className="rating-chart__grid"
            x1={PAD.left}
            x2={W - PAD.right}
            y1={H - PAD.bottom}
            y2={H - PAD.bottom}
          />
          <text className="rating-chart__axis" x={PAD.left - 6} y={PAD.top + 8} textAnchor="end">
            {geo.hi}
          </text>
          <text className="rating-chart__axis" x={PAD.left - 6} y={H - PAD.bottom} textAnchor="end">
            {geo.lo}
          </text>
          <path className="rating-chart__line" d={path} />
          {points.length === 1 && (
            <circle className="rating-chart__dot" cx={geo.xy[0]!.x} cy={geo.xy[0]!.y} r={4} />
          )}
          {h && (
            <>
              <line
                className="rating-chart__crosshair"
                x1={h.x}
                x2={h.x}
                y1={PAD.top}
                y2={H - PAD.bottom}
              />
              <circle className="rating-chart__dot" cx={h.x} cy={h.y} r={4} />
            </>
          )}
        </svg>
        {h && (
          <div
            className="rating-chart__tip"
            style={{ left: `${(h.x / W) * 100}%`, top: `${(h.y / H) * 100}%` }}
            aria-hidden="true"
          >
            <strong>{h.point.rating}</strong>
            <span>{dates.format(new Date(h.point.at))}</span>
          </div>
        )}
      </div>
      <table id={tableId} className="visually-hidden">
        <caption>{t('profile.chart.label', { pool: title })}</caption>
        <thead>
          <tr>
            <th scope="col">{t('profile.chart.date')}</th>
            <th scope="col">{t('leaderboard.col.rating')}</th>
          </tr>
        </thead>
        <tbody>
          {points.map((p) => (
            <tr key={p.at}>
              <td>{dates.format(new Date(p.at))}</td>
              <td>{p.rating}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </figure>
  );
}
