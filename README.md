# Share

For [Horizon](https://playhorizon.dev) (Minecraft: Java Edition 26.3 in Rust). Co-op comforts: no more fighting over drops, and a way to say "here!". English and Arabic.

## How it plays
- **Loot for each player**: a mob you kill with friends near (32 blocks) drops its items once for each of you, each copy only its player can pick up, and its experience goes to each of you. The killer gets one copy. Alone, nothing changes. Spectators and creative players get none.
- **Mark for a friend**: look at a block (up to 128 away) and press **J**: a light beam in your colour and your name show there for everyone for 30 seconds. J on your own mark takes it back; 5 marks each at most.
- Options > Mods > Share turns either off.

## Install
1. Copy `share.hmod` into `%APPDATA%\horizon\horizon-native-mods\`.
2. Add the line `mod.share:true` to `%APPDATA%\horizon\horizon-mods.txt`.
3. Build: the launcher's Mods page **Build**, or `horizon-launcher native-build --instance horizon --trust-native-code`. A native mod runs with your rights: install it only from a source you trust.

The server and every player need the mod. Built for Horizon Nightly `f037744` (2026-11-21). Design and limits: [docs/design.md](docs/design.md).

## Licence
MIT, see [LICENSE](LICENSE). Not affiliated with the Horizon project.
