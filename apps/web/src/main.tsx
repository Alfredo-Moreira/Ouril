import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { registerSW } from 'virtual:pwa-register';

// Bricolage Grotesque with its optical-size axis (display, numbers) and Figtree (interface).
import '@fontsource-variable/bricolage-grotesque/opsz.css';
import '@fontsource-variable/figtree';

import { App } from './App';
import './i18n';
import './styles.css';

const root = document.getElementById('root');
if (!root) throw new Error('#root not found');

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// PWA: precache the app shell + WASM so it works offline after the first load. Same-origin only.
if (import.meta.env.PROD) registerSW({ immediate: true });
