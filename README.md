# Ouril

**Ouril** is a digital version of the traditional Cape Verdean seed-sowing game, part of the worldwide **Oware** (mancala) family. Two players take turns sowing seeds around a board of twelve pits and capturing the opponent's seeds. Whoever captures the most wins.

This monorepo holds every part of the project:

- **Core** (Rust): the rules engine and AI, shared by every app and the server
- **Web** (TypeScript + React): play in the browser, installable and offline
- **iOS** (SwiftUI) and **Android** (Jetpack Compose): native mobile apps (not started)
- **Server** (Rust): accounts and sync (built, not in the MVP), then real-time and async online play

## Status

🚀 **The MVP is ready: Ouril on the web.** Play against the computer in your browser, with a 3D board, four Cape Verdean rule sets, a tutorial and game music. It works offline and needs no account. It's a static site with no server ([ADR 0025](docs/decisions/0025-web-first-static-mvp.md), [deployment](docs/architecture/deployment.md)). The server, sign-in and sync are built and switched on in a later release; iOS and Android haven't started yet ([roadmap](docs/product/roadmap.md)).

To run it locally you need only Docker (and optionally [`just`](https://github.com/casey/just)):

```bash
just env && just bindings && just dev   # then open http://localhost:5173
```

See [CONTRIBUTING.md](CONTRIBUTING.md#local-setup-docker-first) for the setup without `just`.

The first release (MVP) is a standalone web game: play Cape Verdean Ouril against the computer, learn the rules in a tutorial, and pick up an unfinished game where you left it. Later releases add optional sign-in with backup and stats, the iOS and Android apps, online play against others (live and turn-by-turn), more variants and languages, and global and regional leaderboards.

## Documentation

All documentation lives in [`docs/`](docs/README.md). Good places to start:

- [Vision](docs/vision.md): what we're building and why
- [Game rules](docs/game/rules.md): how Cape Verdean Ouril is played
- [Roadmap](docs/product/roadmap.md): from MVP to multiplayer and leaderboards

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md). Questions: [SUPPORT.md](SUPPORT.md). Security issues: [SECURITY.md](SECURITY.md). If the game doesn't match how you play Ouril where you live, please [tell us](https://github.com/Alfredo-Moreira/Ouril/issues/new/choose).

## License

[MIT](LICENSE)
