import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { registerSW } from 'virtual:pwa-register';

import '@fontsource-variable/fraunces';
import '@fontsource-variable/manrope';

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
