import { useEffect, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { createAi, type AiPlayer } from './ai';
import { CoreContext, type CoreContextValue } from './CoreContext';
import { loadCore } from './index';
import type { CoreApi } from './types';

export interface CoreProviderProps {
  children: ReactNode;
  /** Inject a core (tests: the real module via loadCoreSync, or a fake). */
  core?: CoreApi;
  ai?: AiPlayer;
}

/** Loads the WASM core (same-origin asset) and the AI worker, then renders children. */
export function CoreProvider({ children, core: injected, ai: injectedAi }: CoreProviderProps) {
  const { t } = useTranslation();
  const [value, setValue] = useState<CoreContextValue | null>(() =>
    injected ? { core: injected, ai: injectedAi ?? createAi(injected) } : null,
  );
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (value) return;
    let cancelled = false;
    loadCore().then(
      (core) => {
        if (!cancelled) setValue({ core, ai: injectedAi ?? createAi(core) });
      },
      () => {
        if (!cancelled) setFailed(true);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [value, injectedAi]);

  // The AI worker lives as long as the app (one per page), so it is never disposed here.

  if (failed)
    return (
      <p className="error" role="alert">
        {t('common.engine_failed')}
      </p>
    );
  if (!value) return <p role="status">{t('common.loading')}</p>;
  return <CoreContext.Provider value={value}>{children}</CoreContext.Provider>;
}
