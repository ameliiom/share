# Share: design

Co-op comforts for two or more players. 0.1.0: ideas 1 and 2 of `docs/batch-3.md`; 0.2.0: the
four comforts of `docs/batch-4.md` no other mod of the set already gives (picked 2026-11-26).

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

## 4. Batch 4 (0.2.0): what is here, what is elsewhere

Batch 4 (`docs/batch-4.md`) named eight comforts. Four already live in other mods of the set, so
Share does not repeat them (one meaning, one place):

| # | Comfort | Where |
|---|---|---|
| 1 | Friend through walls, edge arrows | **Team** (cards with distance and arrow, edge arrows, V pings) |
| 2 | Friend's hearts and food on screen | **Team** cards |
| 3 | Alert when a friend is hurt | **Team** (card flashes red); a sound and the 4-health rule would be added there |
| 7 | Shared map | **Map** shows teammates already; sharing explored ground belongs in Map |
| 4 | Revive instead of death | Share |
| 5 | Throw to a friend | Share |
| 6 | Pets for both | Share (sit and follow; see below) |
| 8 | Team hit | Share |

- **Revive**: with another playing player online, a player whose health runs out goes down for
  30 s instead: health 1, slowness, weakness, resistance V (nothing hurts them but `/kill` and the
  void), mining fatigue, and their blows do nothing. Everyone reads it in the action bar. A
  friend sneaking within 2 blocks for 3 s brings them up at 4 health; stepping away resets it.
  Time up, `/kill`, the void, creative, spectator, or no friend online: normal death. A downed
  player who logs out is no longer down.
- **Throw**: Q while a friend's body is within 16 blocks ahead and 0.9 off your line of sight: the
  item appears at their feet, theirs alone (`ItemEntity.target`), with no pickup delay, so it goes
  into their inventory at once, or waits there when it is full.
- **Pets**: a friend's wolf, cat or parrot sits or stands on your empty-hand click; while its
  owner is offline, elsewhere or 48+ blocks off, it walks to you (4–12 blocks) or appears beside
  you (12–48), as it does for its owner. The tamer stays the owner. Tamed horses are anyone's to
  ride already. **Not in**: wolves defending a friend. A set target, and the wolf's own anger
  (`angry_at`, `anger_end_time`), both left a tamed wolf idle next to the foe; it fights only for
  its owner (vanilla).
- **Team hit**: a player's blow on a mob a friend hit within 2 s does 25% more. Never on players
  or tamed animals.
- Each has its own setting (Options > Mods > Share), on by default.

## 5. Measured in the game (Horizon f037744, one player, 2026-11-26)

One account only, so the sharing path was run with a test build that counted one extra player
(solo shared with yourself), then the real build checked solo play.

| Case | Result |
|---|---|
| Zombie with a shovel (drop chance 2.0) killed by the player, test build | exactly **1** shovel on the ground, its `Owner` the killer's UUID; the killer picks it up |
| Its experience, test build | 5 points given straight to the killer, **no** orbs in the world |
| J on a block 8 away | beam in the player's colour, the block outlined, the name above it (text display `{color:"#FF55FF", text:"YNGI4654"}`) |
| J again on that mark | gone (text display removed) |
| Waiting | the mark there at 24 s, gone by about 30 s |

| 0.2.0, alone: a zombie's shovel | 1 shovel, no `Owner`, 3 orbs: solo play is vanilla |
| 0.2.0, alone: 1000 damage | normal death (no friend online, no down) |
| 0.2.0, alone: Q on dirt | dropped as usual, no owner |
| 0.2.0, a wolf whose owner is offline: empty-hand click | `Sitting` 0b → 1b; again → 0b |
| 0.2.0, same wolf, player walks 25 blocks off | the wolf appears beside the player within 1.5 s |
| 0.2.0, same wolf, player 6 blocks off | it walks toward the player |

Not tested in the game (needs a second account): two real players getting one copy each, a
friend unable to pick up your copy, a friend seeing your beam and name; going down and being
revived; throwing to a friend; team hit's extra damage.

## 6. Rules tested (`src/rules.rs`, `src/coop.rs`, `cargo test`)

Solo stays vanilla · killer once and first · far, spectating and creative players get none · a far
killer still gets theirs · a spectating killer gets none · players, armor stands and unhit deaths
not shared · one copy of each drop per recipient, owned drops once · your own mark removed, a
friend's not · the sixth mark replaces your oldest · marks go at 30 s · same spot in another
dimension is a new mark · beam and fade geometry. · a player goes down only with a friend online · `/kill`, the void, creative and a second
death are final · 3 s of sneaking revives · stepping away resets · 30 s alone is death · a throw
goes to the nearest friend looked at, not past reach, behind or aside · team hit needs a friend's
blow within 2 s, never on players or pets · pets follow a friend only while the owner is away.
