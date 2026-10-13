import { useTranslation } from 'react-i18next';

import type { MoveEvent } from '../engine/types';
import { describeEvents } from '../game/describeEvents';

export function MoveAnnouncer({
  events,
  byHuman,
  opponent,
}: {
  events: MoveEvent[] | null;
  byHuman: boolean;
  /** The other player's name in online games; absent = the computer. */
  opponent?: string;
}) {
  const { t } = useTranslation();
  const lines = events ? describeEvents(t, events, byHuman, opponent) : [];
  return (
    <p className="announcer" aria-live="polite" aria-atomic="true">
      {lines.join(' ')}
    </p>
  );
}
