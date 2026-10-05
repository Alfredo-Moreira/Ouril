/**
 * The About page's content: who makes Ouril, where to find him, and how to buy him a coffee.
 * Fill in the blanks here. Anything left empty is hidden in production builds (development
 * shows a placeholder so the layout can be checked). Text shown to players lives in
 * `shared/i18n/en.json` (`about.*`).
 */
import photo from '../assets/alfredo.avif';

export type SocialKind =
  'email' | 'linkedin' | 'github' | 'instagram' | 'facebook' | 'twitch' | 'website' | 'x';

export const DEVELOPER = {
  name: 'Alfredo Moreira',
  photo,
  /** Profile links, in display order. `url` empty = not shown. */
  socials: [
    { kind: 'website', url: 'https://alfredomoreira.dev' },
    { kind: 'email', url: 'mailto:devops.moreira@gmail.com' },
    { kind: 'linkedin', url: 'https://www.linkedin.com/in/alfredomoreira/' },
    { kind: 'github', url: 'https://github.com/Alfredo-Moreira' },
    { kind: 'instagram', url: 'https://www.instagram.com/alfredo.r.moreira/' },
    { kind: 'facebook', url: 'https://www.facebook.com/alfredo.d.moreira' },
    { kind: 'twitch', url: 'https://www.twitch.tv/playbydev' },
  ] satisfies { kind: SocialKind; url: string }[],
  /** "Buy me a coffee": usernames without @ or $. Empty = that option isn't shown. */
  coffee: {
    /** Pre-filled amount, in US dollars, wherever the service's link supports it. */
    amountUsd: 5,
    /** paypal.me username: https://paypal.me/<username>/<amount>USD */
    paypal: 'devopsmoreira',
    /** Venmo username: https://venmo.com/<username>?txn=pay&amount=<amount> */
    venmo: 'cv_badiu',
    /** Cash App $cashtag: https://cash.app/$<cashtag>/<amount> */
    cashapp: 'cvbadiu007',
    /** The email or phone registered with Zelle (Zelle has no web link, so no amount). */
    zelle: 'amoreira20149@gmail.com',
  },
};
