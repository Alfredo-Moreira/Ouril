import { FEATURES } from '../features';

export type PlayMode = 'computer' | 'online';

/** Who you can play: the computer always, other players online when accounts are on. */
export const PLAY_MODES: PlayMode[] = FEATURES.accounts ? ['computer', 'online'] : ['computer'];
