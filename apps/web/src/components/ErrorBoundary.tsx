/**
 * Catches a crash in one screen so the app shows a way out instead of an empty page.
 * Reset by changing `resetKey` (the route), so navigating away clears the error.
 */
import { Component, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

interface Props {
  resetKey: string;
  children: ReactNode;
}

export class ScreenErrorBoundary extends Component<Props, { error: boolean; key: string }> {
  override state = { error: false, key: this.props.resetKey };

  static getDerivedStateFromError() {
    return { error: true };
  }

  static getDerivedStateFromProps(props: Props, state: { error: boolean; key: string }) {
    return props.resetKey !== state.key ? { error: false, key: props.resetKey } : null;
  }

  override componentDidCatch(error: unknown) {
    console.error('Screen crashed', error);
  }

  override render() {
    return this.state.error ? <ScreenError /> : this.props.children;
  }
}

function ScreenError() {
  const { t } = useTranslation();
  return (
    <section className="card screen-error" role="alert">
      <h1>{t('error.screen.title')}</h1>
      <p>{t('error.screen.body')}</p>
      <div className="row">
        <button type="button" className="button button--primary" onClick={() => location.reload()}>
          {t('update.reload')}
        </button>
        <Link className="button" to="/">
          {t('error.screen.home')}
        </Link>
      </div>
    </section>
  );
}
