# Pirates!

A clone of Sid Meier's *Pirates!* (MicroProse, 1987) written in Rust with
[macroquad](https://macroquad.rs). Everything — the map, the art, the music —
is generated in code; there are no asset files. The original's copy
protection (manual questions) is deliberately not included.

## Building and running

```sh
cargo run --release
```

## Features

- **Six eras** (1560–1680) plus six **famous expeditions** (Drake, Piet Heyn,
  L'Olonnais, Morgan, De Graaf, de Pointis). Serve Spain, England, France
  or Holland, at four difficulty levels, with one of five special skills.
- **The Caribbean**: a coastline map with 42 historical towns whose owners
  change by era. Sailing depends on the wind and your rig. Deep-draft ships
  can run aground in the shallows, and storms blow through in hurricane season.
- **A living world**: merchants, warships, smugglers, pirates and pirate
  hunters sail between ports. Wars and treaties change month to month,
  silver trains arrive at the treasure ports, and the treasure fleet sails
  every May.
- **Towns**: governors (letters of marque, promotions from Ensign to Duke,
  land grants, their daughters), taverns (recruiting, gossip, mysterious
  strangers, treasure map pieces), merchants (four trade goods, food,
  cannon), shipwrights, and dividing the plunder.
- **Naval battles**: top-down broadside combat with wind, reloading,
  boarding, and assaults on forts.
- **Sword fights**: rapier, longsword or cutlass, with high, middle and low
  attacks and parries, while the two crews fight in the background.
- **Land assaults**: tactical battles through jungle and hills against the
  garrison.
- **Long-term goals**: find your four lost relatives, dig up buried treasure,
  marry, earn titles, and survive the years until you retire. Your career is
  scored at the end.
- Save and load, procedural PC-speaker sound and music, and mouse support
  in menus.

## Controls

| Where | Keys |
|---|---|
| Menus | Arrows + Enter (or number keys / mouse), Esc back |
| Sailing | Left/Right steer, M map, I info, D dig, F fast, Space pause, Esc menu |
| Naval battle | Left/Right turn, Up full sail, Down battle sail, Space fire |
| Fencing | Up/Down guard height, Space/Right attack, Left leap back |
| Land battle | Tab/1–3 select group, arrows move, Space fire, Esc retreat |
| Merchant | Up/Down select, Right/B buy, Left/S sell, Shift ×10 |

## Development

- `cargo test` runs headless tests of the game flow.
- `PIRATES_SCRIPT=file` drives the game from a script (`wait N`,
  `press KEY…`, `hold KEY N`, `type TEXT`, `shot file.png`, `quit`), for
  automated testing and screenshots.
- `PIRATES_TEST=naval|fencing|land|map` opens a single minigame directly.
- `PIRATES_DEBUG=ship,pirate,warship,rich,hostile,old` sets up test
  scenarios when a new career starts.
- `PIRATES_MUTE=1` turns off sound.
