# Share: design

Co-op comforts for two or more players: loot that isn't fought over, and a way to point at a
spot. Ideas 1 and 2 of `docs/batch-3.md` (picked 2026-11-26).

## 1. Outcome

Playing together, nobody grabs the other's drops, and "come here" is one key instead of
coordinates in chat.

## 2. Loot for each player

- A mob that dies with a player's hit on it in the last 100 ticks drops its items **once for each
  player** in survival or adventure, in its level, within **32 blocks**, and the killer wherever
  they stand. Each copy carries its player as the item entity's target (`ItemDrop::target`,
  vanilla `ItemEntity.setTarget`): only that player can pick it up. Copies with different
  targets never merge.
- The killer gets **one** copy, never two (the recipient list is a set, killer first).
- Its **experience** goes to each of those players: the orbs would fly to whoever stands nearest,
  so with two or more players the orbs are not dropped and each player is given the same points
  (`give_experience_points`, which mends as an orb does) on the next tick.
- **Solo play stays vanilla**: with one recipient nothing changes. Also untouched: a player's own
  death (their inventory), armor stands, mobs no player hurt (lava, farms of traps), drops that
  already have an owner, spectators and creative players, structure chests, fishing and mining.
- Setting **Loot for each player near** (server) turns it off.

Known in v0.1: everyone sees every copy (only the owner can pick theirs up); a mob's drops include
items it picked up (a fox's mouth item), which are copied as well.

## 3. Mark for a friend

- **J** (rebindable; B, Y, Z, U are taken by Horizon and the other mods) on a block up to **128**
  away places a mark: a light beam 64 blocks high in your colour, the block outlined, and your
  name above it, seen by every player in that dimension for **30 seconds** (fading the last 3).
- J on your own mark (that block or one beside it) takes it back. At most **5** per player; a
  sixth replaces your oldest.
- The client sends the block it looks at; the server checks the reach, keeps the marks, sends each
  player the marks of their dimension (again when they join or change dimension), and summons a
  vanilla text display with the name (tagged `share.mark`; one left from a stopped server is
  removed when it loads).
- The beam is drawn by the client (`render_level_stage`: quads, the outline and a core line seen
  through hills).
- Setting **Marks for friends** (server) turns it off.

## 4. Measured in the game (Horizon f037744, one player, 2026-11-26)

One account only, so the sharing path was run with a test build that counted one extra player
(solo shared with yourself), then the real build checked solo play.

| Case | Result |
|---|---|
| Zombie with a shovel (drop chance 2.0) killed by the player, test build | exactly **1** shovel on the ground, its `Owner` the killer's UUID; the killer picks it up |
| Its experience, test build | 5 points given straight to the killer, **no** orbs in the world |
| J on a block 8 away | beam in the player's colour, the block outlined, the name above it (text display `{color:"#FF55FF", text:"YNGI4654"}`) |
| J again on that mark | gone (text display removed) |
| Waiting | the mark there at 24 s, gone by about 30 s |

Not tested in the game (needs a second account): two real players getting one copy each, a
friend unable to pick up your copy, a friend seeing your beam and name.

## 5. Rules tested (`src/rules.rs`, `cargo test`)

Solo stays vanilla · killer once and first · far, spectating and creative players get none · a far
killer still gets theirs · a spectating killer gets none · players, armor stands and unhit deaths
not shared · one copy of each drop per recipient, owned drops once · your own mark removed, a
friend's not · the sixth mark replaces your oldest · marks go at 30 s · same spot in another
dimension is a new mark · beam and fade geometry.
