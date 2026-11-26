# Share

For [Horizon](https://playhorizon.dev) (Minecraft: Java Edition 26.3 in Rust). Co-op comforts: no more fighting over drops, a friend to pick you up, items handed over, pets for both. English and Arabic.

## How it plays
- **Loot for each player**: a mob you kill with friends near (32 blocks) drops its items once for each of you, each copy only its player can pick up, and its experience goes to each of you. The killer gets one copy. Alone, nothing changes. Spectators and creative players get none.
- **Revive instead of death**: with a friend online, when your health runs out you are down for 30 seconds instead of dead. A friend who sneaks next to you for 3 seconds brings you back. Alone, `/kill` or the void: normal death.
- **Throw to a friend**: press Q while looking at a friend (up to 16 blocks): the item goes straight into their inventory, or waits at their feet, only for them.
- **Pets for both**: your friend's wolf, cat or parrot sits or stands when you click it with an empty hand, and follows you while its owner is away. The tamer stays its owner.
- **Team hit**: hitting a mob your friend hit in the last 2 seconds does 25% more damage.
- Options > Mods > Share turns each one off.

Pings, friend locations, health cards and hurt alerts come from [Team](https://github.com/ameliiom/team), and the minimap from [Map](https://github.com/ameliiom/map).

## Changes
- **0.2.1**: the J marks are retired: Team's V pings do the same (one feature, one place). A mark label left in a saved world is removed when it loads.
- **0.2.0**: revive instead of death, throw to a friend, pets for both, team hit.
- **0.1.0**: loot for each player near, J marks.

## Install
Needs [Kit](https://github.com/ameliiom/kit) 0.1 or later, the shared code of these mods, installed the same way (`kit.hmod`, `mod.kit:true`).

1. Copy `share.hmod` into `%APPDATA%\horizon\horizon-native-mods\`.
2. Add the line `mod.share:true` to `%APPDATA%\horizon\horizon-mods.txt`.
3. Build: the launcher's Mods page **Build**, or `horizon-launcher native-build --instance horizon --trust-native-code`. A native mod runs with your rights: install it only from a source you trust.

The server and every player need the mod. Built for Horizon Nightly `f037744` (2026-11-21). Design and limits: [docs/design.md](docs/design.md).

## Licence
MIT, see [LICENSE](LICENSE). Not affiliated with the Horizon project.
