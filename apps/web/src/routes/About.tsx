/**
 * About the developer: photo, a short introduction, social links and "buy me a coffee"
 * (PayPal, Venmo, Zelle). Content comes from `content/developer.ts`; anything not filled in is
 * hidden in production and shown as a placeholder in development.
 */
import { useEffect, useState, type ComponentType, type SVGProps } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation } from 'react-router';

import { BuyBoards } from '../components/BuyBoards';

import {
  CoffeeIcon,
  CopyIcon,
  FacebookIcon,
  GitHubIcon,
  GlobeIcon,
  InstagramIcon,
  LinkedInIcon,
  MailIcon,
  TwitchIcon,
  XIcon,
} from '../components/Icons';
import { DEVELOPER, type SocialKind } from '../content/developer';

const SOCIAL_ICONS: Record<SocialKind, ComponentType<SVGProps<SVGSVGElement>>> = {
  website: GlobeIcon,
  github: GitHubIcon,
  linkedin: LinkedInIcon,
  instagram: InstagramIcon,
  x: XIcon,
  email: MailIcon,
  facebook: FacebookIcon,
  twitch: TwitchIcon,
};

/** Unfilled entries show as placeholders only in development builds. */
const SHOW_PLACEHOLDERS = import.meta.env.DEV;

export function About() {
  const { t } = useTranslation();
  const location = useLocation();
  // `/about#buy` (the header's "Buy a board"): scroll to that section, also when already here.
  useEffect(() => {
    if (location.hash !== '#buy') return;
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    document.getElementById('buy')?.scrollIntoView({
      behavior: reduced ? 'auto' : 'smooth',
      block: 'start',
    });
  }, [location.hash, location.key]);
  const socials = DEVELOPER.socials.filter((s) => s.url || SHOW_PLACEHOLDERS);
  const { amountUsd, paypal, venmo, cashapp, zelle } = DEVELOPER.coffee;
  const note = t('about.coffee.note');
  const amount = t('about.coffee.amount', { amount: amountUsd });
  // Each service's own "pay this person" link, with the amount pre-filled where supported.
  const links = [
    {
      key: 'paypal',
      handle: paypal ? `@${paypal}` : '',
      href: paypal ? `https://paypal.me/${encodeURIComponent(paypal)}/${amountUsd}USD` : '',
    },
    {
      key: 'venmo',
      handle: venmo ? `@${venmo}` : '',
      href: venmo
        ? `https://venmo.com/${encodeURIComponent(venmo)}?txn=pay&amount=${amountUsd}&note=${encodeURIComponent(note)}`
        : '',
    },
    {
      key: 'cashapp',
      handle: cashapp ? `$${cashapp}` : '',
      href: cashapp ? `https://cash.app/$${encodeURIComponent(cashapp)}/${amountUsd}` : '',
    },
  ].filter((l) => l.href || SHOW_PLACEHOLDERS);
  const coffee = links.length > 0 || zelle || SHOW_PLACEHOLDERS;

  return (
    <article className="about">
      <header className="about__hero">
        <img
          className="about__photo"
          src={DEVELOPER.photo}
          alt={t('about.photo_alt', { name: DEVELOPER.name })}
          width={640}
          height={640}
          decoding="async"
        />
        <div>
          <p className="about__kicker">{t('about.kicker')}</p>
          <h1 className="about__name">{DEVELOPER.name}</h1>
          <p className="about__intro">{t('about.intro')}</p>
          <p className="about__intro about__more">{t('about.more')}</p>
        </div>
      </header>

      {socials.length > 0 && (
        <section className="card about__section" aria-labelledby="about-find">
          <h2 id="about-find" className="card__title">
            {t('about.find_me')}
          </h2>
          <ul className="socials">
            {socials.map(({ kind, url }) => {
              const Icon = SOCIAL_ICONS[kind];
              const label = t(`about.social.${kind}`);
              return (
                <li key={kind}>
                  {url ? (
                    <a
                      className="social"
                      href={url}
                      target={kind === 'email' ? undefined : '_blank'}
                      rel="noopener noreferrer"
                    >
                      <Icon />
                      <span>{label}</span>
                    </a>
                  ) : (
                    <span className="social social--placeholder" title={t('about.placeholder')}>
                      <Icon />
                      <span>{label}</span>
                    </span>
                  )}
                </li>
              );
            })}
          </ul>
        </section>
      )}

      {coffee && (
        <section className="card card--accent about__section coffee" aria-labelledby="about-coffee">
          <h2 id="about-coffee" className="card__title">
            <CoffeeIcon />
            {t('about.coffee.title')}
          </h2>
          <p>{t('about.coffee.body')}</p>
          <div className="coffee__options">
            {links.map((l) => (
              <PayLink
                key={l.key}
                label={t(`about.coffee.${l.key}`)}
                href={l.href}
                handle={l.handle}
                amount={amount}
              />
            ))}
            {zelle || SHOW_PLACEHOLDERS ? <ZelleHandle handle={zelle} amount={amount} /> : null}
          </div>
        </section>
      )}
      <BuyBoards id="buy" />
    </article>
  );
}

function PayLink({
  label,
  href,
  handle,
  amount,
}: {
  label: string;
  href: string;
  handle: string;
  amount: string;
}) {
  const { t } = useTranslation();
  if (!href) {
    return (
      <span className="pay pay--placeholder" title={t('about.placeholder')}>
        <span className="pay__name">{label}</span>
        <span className="pay__handle">{t('about.placeholder')}</span>
      </span>
    );
  }
  return (
    <a className="pay" href={href} target="_blank" rel="noopener noreferrer">
      <span className="pay__name">{label}</span>
      <span className="pay__handle">{handle}</span>
      <span className="pay__amount">{t('about.coffee.send', { amount })}</span>
    </a>
  );
}

/** Zelle has no public web link: show the email or phone, with a copy button. */
function ZelleHandle({ handle, amount }: { handle: string; amount: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  if (!handle) {
    return (
      <span className="pay pay--placeholder" title={t('about.placeholder')}>
        <span className="pay__name">{t('about.coffee.zelle')}</span>
        <span className="pay__handle">{t('about.placeholder')}</span>
      </span>
    );
  }
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(handle);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard blocked: the handle is visible and selectable anyway.
    }
  };
  return (
    <div className="pay">
      <span className="pay__name">{t('about.coffee.zelle')}</span>
      <span className="pay__handle">{handle}</span>
      <span className="pay__amount">{t('about.coffee.send', { amount })}</span>
      <button type="button" className="button button--ghost pay__copy" onClick={() => void copy()}>
        <CopyIcon />
        {copied ? t('about.coffee.copied') : t('about.coffee.copy')}
      </button>
      <span className="visually-hidden" role="status">
        {copied ? t('about.coffee.copied') : ''}
      </span>
    </div>
  );
}
