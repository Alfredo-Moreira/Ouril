/**
 * The privacy policy (`/privacy`) and the account-deletion page (`/delete-account`).
 * Text lives in shared/i18n/en.json: `privacy.<section>.title` and `privacy.<section>.<n>`;
 * section ids and paragraph counts must match those keys (checked by i18n/keys.test.ts).
 * Keep it in step with what the app and server actually do (docs/product/features/accounts.md,
 * docs/architecture/backend.md, ADR 0014, ADR 0026), and bump `updated` with every change.
 */

export const PRIVACY = {
  /** The person responsible for the data (the GDPR "controller"). */
  controller: 'Alfredo Ramos Moreira',
  /** Where privacy questions and deletion requests go. */
  email: 'devops.moreira@gmail.com',
  /** Date of the last change, ISO `YYYY-MM-DD`. */
  updated: '2026-10-14',
  /** Longest time deleted data can survive in database backups. */
  backupDays: 30,
} as const;

export interface PrivacySection {
  id: string;
  paragraphs: number;
}

export const PRIVACY_SECTIONS: readonly PrivacySection[] = [
  { id: 'short', paragraphs: 3 },
  { id: 'who', paragraphs: 2 },
  { id: 'guest', paragraphs: 2 },
  { id: 'account', paragraphs: 6 },
  { id: 'usage', paragraphs: 3 },
  { id: 'use', paragraphs: 2 },
  { id: 'processors', paragraphs: 6 },
  { id: 'retention', paragraphs: 3 },
  { id: 'rights', paragraphs: 2 },
  { id: 'children', paragraphs: 1 },
  { id: 'changes', paragraphs: 1 },
];
