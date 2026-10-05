import { createContext, useContext } from 'react';

import type { AiPlayer } from './ai';
import type { CoreApi } from './types';

export interface CoreContextValue {
  core: CoreApi;
  ai: AiPlayer;
}

export const CoreContext = createContext<CoreContextValue | null>(null);

/** The loaded engine and AI. Only use below `<CoreProvider>` (it renders after the core loads). */
export function useCore(): CoreContextValue {
  const value = useContext(CoreContext);
  if (!value) throw new Error('useCore() outside <CoreProvider>');
  return value;
}
