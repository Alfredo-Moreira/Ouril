# Security policy

> How to report a vulnerability in Ouril, and what's in scope. **Status:** Draft

## Reporting a vulnerability

**Please don't open a public issue.** Report it privately through GitHub:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability** ([direct link](https://github.com/Alfredo-Moreira/Ouril/security/advisories/new)).
3. Describe the issue, how to reproduce it, and its impact. A proof of concept helps.

<!-- TODO(owner): enable "Private vulnerability reporting" in Settings → Code security. Optionally add a backup contact: security@TODO.example -->

We aim to acknowledge reports within **7 days** and to keep you updated until it's fixed. We'll credit you in the advisory unless you'd rather stay anonymous. Please give us reasonable time to release a fix before disclosing publicly.

## Supported versions

Ouril is in pre-development and has no releases yet. Once it does, security fixes go to the **latest release** of each app and to the server. Players below the server's `min_supported` app version are asked to update (see [versioning](docs/architecture/versioning.md#old-apps)).

## Scope

In scope:

- **Server** (`apps/server`): the HTTP API, realtime connections, authorization between players.
- **Accounts and sign-in:** Google and Apple OAuth flows, sessions and tokens ([accounts](docs/product/features/accounts.md)).
- **Sync** (`core/sync` and the app sync clients): tampering with another player's data, replaying or forging mutations.
- **Multiplayer integrity:** submitting illegal moves, or affecting ratings and leaderboards, despite the server being authoritative.
- **Privacy:** leaking personal data, or guests making network requests without telemetry consent ([ADR 0014](docs/decisions/0014-telemetry-consent.md)).
- **Apps** (iOS, Android, web): local data exposure, unsafe deep links, XSS in the web app.
- **Supply chain:** this repository's CI workflows and published artifacts.

Out of scope:

- Beating the computer opponent, or rule disagreements: use the [rule or variant report](https://github.com/Alfredo-Moreira/Ouril/issues/new/choose) form.
- Issues in third-party services (Google, Apple, app stores, hosting) unless caused by how we use them.
- Denial of service through traffic volume, social engineering, and physical attacks.
- Reports from automated scanners without a demonstrated impact.
