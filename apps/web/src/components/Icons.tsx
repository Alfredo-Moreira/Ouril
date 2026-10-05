/**
 * Small inline SVG icons (no icon font, no network). Decorative: always `aria-hidden`; the
 * accessible name comes from the visible text next to them.
 */
import type { SVGProps } from 'react';

type IconProps = SVGProps<SVGSVGElement>;

function Svg(props: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width="20"
      height="20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...props}
    />
  );
}

/** The Ouril mark: a pit with three seeds. */
export function LogoMark(props: IconProps) {
  return (
    <svg viewBox="0 0 32 32" width="28" height="28" aria-hidden="true" focusable="false" {...props}>
      <circle cx="16" cy="16" r="15" fill="currentColor" opacity="0.14" />
      <circle cx="16" cy="16" r="11" fill="none" stroke="currentColor" strokeWidth="2" />
      <ellipse cx="12.4" cy="17.4" rx="3.1" ry="2.5" fill="currentColor" />
      <ellipse cx="19" cy="18.6" rx="3.1" ry="2.5" fill="currentColor" opacity="0.85" />
      <ellipse cx="16.4" cy="12.2" rx="3.1" ry="2.5" fill="currentColor" opacity="0.7" />
    </svg>
  );
}

export const PlayIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="9" />
    <path d="M10 8.5v7l5.5-3.5z" fill="currentColor" stroke="none" />
  </Svg>
);

export const LearnIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M3 7.5 12 4l9 3.5-9 3.5z" />
    <path d="M7 9.5v4.5c0 1.4 2.2 2.5 5 2.5s5-1.1 5-2.5V9.5" />
    <path d="M21 7.5V13" />
  </Svg>
);

export const RulesIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M6 3.5h9l3 3V20.5H6z" />
    <path d="M9 10h6M9 13.5h6M9 17h4" />
  </Svg>
);

export const StatsIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" />
  </Svg>
);

export const SettingsIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  </Svg>
);

export const UserIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="12" cy="8.5" r="3.5" />
    <path d="M5 20c1.2-3.4 3.8-5 7-5s5.8 1.6 7 5" />
  </Svg>
);

export const ArrowIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M5 12h14M13 6l6 6-6 6" />
  </Svg>
);

export const UndoIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 14 4 9l5-5" />
    <path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" />
  </Svg>
);

export const BulbIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 18h6M10 21h4" />
    <path d="M12 3a6 6 0 0 0-3.6 10.8c.6.5 1 1.2 1.1 2V16h5v-.2c.1-.8.5-1.5 1.1-2A6 6 0 0 0 12 3z" />
  </Svg>
);

export const HomeIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M4 10.5 12 4l8 6.5V20h-5v-5.5h-6V20H4z" />
  </Svg>
);

export const FullscreenIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
  </Svg>
);

export const ExitFullscreenIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" />
  </Svg>
);

/** A five-pointed star, as on the Cape Verdean flag. */
export const StarIcon = (p: IconProps) => (
  <Svg {...p}>
    <path
      d="m12 3 2.6 5.6 6.1.7-4.5 4.2 1.2 6L12 16.6 6.6 19.5l1.2-6-4.5-4.2 6.1-.7z"
      fill="currentColor"
      stroke="none"
    />
  </Svg>
);

/** An ouri seed. */
export const SeedIcon = (p: IconProps) => (
  <Svg {...p}>
    <ellipse cx="12" cy="12.5" rx="6.5" ry="5.2" fill="currentColor" stroke="none" opacity="0.9" />
    <path d="M9.5 10.5c1-.9 2.4-1.3 3.8-1" stroke="#fff" strokeOpacity="0.55" />
  </Svg>
);

export const PauseIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 5v14M15 5v14" />
  </Svg>
);

/** A white flag: forfeiting a game. */
export const FlagIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M5 21V4" />
    <path d="M5 4.5c4-2 7 2 11 0 1-.5 2-.6 3-.5v9c-1-.1-2 0-3 .5-4 2-7-2-11 0" />
  </Svg>
);

export const InfoIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5.5M12 7.6v.1" />
  </Svg>
);

export const GlobeIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18M12 3c2.6 2.6 3.8 5.6 3.8 9s-1.2 6.4-3.8 9c-2.6-2.6-3.8-5.6-3.8-9S9.4 5.6 12 3z" />
  </Svg>
);

export const GitHubIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 19c-4 1.3-4-2-6-2.5M15 21v-3.4a3 3 0 0 0-.8-2.3c2.7-.3 5.5-1.3 5.5-6a4.6 4.6 0 0 0-1.3-3.2 4.3 4.3 0 0 0-.1-3.2s-1-.3-3.4 1.3a11.6 11.6 0 0 0-6 0C6.5 2.6 5.5 2.9 5.5 2.9a4.3 4.3 0 0 0-.1 3.2A4.6 4.6 0 0 0 4 9.3c0 4.6 2.8 5.6 5.5 6a3 3 0 0 0-.8 2.3V21" />
  </Svg>
);

export const LinkedInIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="3" y="3" width="18" height="18" rx="3" />
    <path d="M8 10.5V16M8 7.6v.1M12 16v-3.2c0-1.4 1-2.3 2.2-2.3s2 .9 2 2.3V16M12 10.5V16" />
  </Svg>
);

export const InstagramIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="3.5" y="3.5" width="17" height="17" rx="5" />
    <circle cx="12" cy="12" r="3.8" />
    <path d="M17.2 6.8v.1" />
  </Svg>
);

export const XIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M4.5 4h4.2L19.5 20h-4.2zM19 4l-6.1 7M5 20l6.1-7" />
  </Svg>
);

export const MailIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="3" y="5" width="18" height="14" rx="2.5" />
    <path d="m4 7 8 6 8-6" />
  </Svg>
);

export const CoffeeIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M4 9h12v5a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5z" />
    <path d="M16 10.5h1.5a2.5 2.5 0 0 1 0 5H16M8 3.5c-.6.8-.6 1.7 0 2.5M11.5 3.5c-.6.8-.6 1.7 0 2.5" />
  </Svg>
);

export const SparkleIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M12 3.5 13.8 9l5.7 1.8-5.7 1.8L12 18.5l-1.8-5.9-5.7-1.8L10.2 9z" />
    <path d="M18.5 3.5v3M17 5h3" />
  </Svg>
);

export const CopyIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="8.5" y="8.5" width="11" height="11" rx="2" />
    <path d="M15.5 8.5V6a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 6v8A1.5 1.5 0 0 0 6 15.5h2.5" />
  </Svg>
);

export const FacebookIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M14.5 21v-7.5h2.6l.4-3H14.5V8.6c0-.9.3-1.5 1.6-1.5h1.6V4.4c-.3 0-1.2-.1-2.3-.1-2.3 0-3.9 1.4-3.9 4V10.5H9v3h2.5V21" />
  </Svg>
);

export const TwitchIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M5 3.5 3.5 7v12h4v2h2.5l2-2h3.5l5-5V3.5z" />
    <path d="M11 8v4.5M15.5 8v4.5" />
  </Svg>
);

export const BagIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M5 8h14l-1.2 12H6.2z" />
    <path d="M9 8V6.5a3 3 0 0 1 6 0V8" />
  </Svg>
);

/** Music note: game music is on (press to pause). */
export const MusicIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 18V5l11-2v13" />
    <circle cx="6.5" cy="18" r="2.5" />
    <circle cx="17.5" cy="16" r="2.5" />
  </Svg>
);

/** Music note crossed out: game music is paused (press to play). */
export const MusicOffIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M9 13V5l11-2v9" />
    <circle cx="6.5" cy="18" r="2.5" />
    <path d="M3 3l18 18" />
  </Svg>
);

/** Next track. */
export const SkipIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M5 5l10 7-10 7z" fill="currentColor" stroke="none" />
    <path d="M19 5v14" />
  </Svg>
);
