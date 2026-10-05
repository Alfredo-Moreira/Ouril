import { useTranslation } from 'react-i18next';

import type { MoveEvent } from '../engine/types';
import { describeEvents } from '../game/describeEvents';

export function MoveAnnouncer({
  events,
  byHuman,
}: {
  events: MoveEvent[] | null;
  byHuman: boolean;
}) {
  const { t } = useTranslation();
  const lines = events ? describeEvents(t, events, byHuman) : [];
  return (
    <p className="announcer" aria-live="polite" aria-atomic="true">
      {lines.join(' ')}
    </p>
  );
}
