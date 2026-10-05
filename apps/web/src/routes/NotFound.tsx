import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

export function NotFound() {
  const { t } = useTranslation();
  return (
    <section>
      <h1>{t('common.not_found')}</h1>
      <Link to="/">{t('common.go_home')}</Link>
    </section>
  );
}
