# Ouril

**Ouril** is a digital version of the traditional Cape Verdean seed-sowing game, part of the worldwide **Oware** (mancala) family. Two players take turns sowing seeds around a board of twelve pits and capturing the opponent's seeds. Whoever captures the most wins.

This monorepo will hold every part of the project:

- **Core** (Rust): the rules engine and AI, shared by every app and the server
- **iOS** (SwiftUI) and **Android** (Jetpack Compose): native mobile apps
- **Web** (TypeScript): play in the browser
- **Server** (Rust): accounts first, then real-time and async online play

## Status

🚧 **Phase 1 (MVP) starting.** The documentation is complete and the Cape Verdean rules are specified with test cases. Implementation begins with the Rust rules engine.

The first release (MVP) is a standalone game: play Cape Verdean Ouril against the computer, learn the rules in a tutorial, and track your stats on your device. You can optionally sign in with Google or Apple to back up your progress. Later releases add online play against others (live and turn-by-turn), more Oware variants, more languages, and global and regional leaderboards.

## Documentation

All documentation lives in [`docs/`](docs/README.md). Good places to start:

- [Vision](docs/vision.md): what we're building and why
- [Game rules](docs/game/rules.md): how Cape Verdean Ouril is played
- [Roadmap](docs/product/roadmap.md): from MVP to multiplayer and leaderboards

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md). Questions: [SUPPORT.md](SUPPORT.md). Security issues: [SECURITY.md](SECURITY.md). If the game doesn't match how you play Ouril where you live, please [tell us](https://github.com/Alfredo-Moreira/Ouril/issues/new/choose).

## License

[MIT](LICENSE)
