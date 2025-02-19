# curseofrust

Curseofrust is [curseofwar](https://github.com/a-nikolaev/curseofwar) (Real Time Strategy Game for Linux) re-implemented in Rust.

The game supports both singleplayer and multiplayer, along with different platforms and networking protocols.

## Common Crates

- **`curseofrust`**: The root crate supports the game (grid, bots, etc.).
- `curseofrust-net-foundation`: Bare networking layer on the top of [`unisock`](https://codeberg.org/DM-Earth/unisock).
- `curseofrust-msg`: Curseofwar messaging protocol implementation.
- `curseofrust-cli-parser`: Simple CLI arguments parser for curseofrust.

## Platforms

- `curseofrust-console`: TUI/CLI implementation. Supports multiplayer.
- `curseofrust-gui-cocoa`: GUI implementation based on Cocoa for macOS. Currently does not support multiplayer. It is now hosted [here](https://codeberg.org/DM-Earth/curseofrust-gui-cocoa).
- `curseofrust-server`: The dedicated server implementation with a CLI interface.

## Protocols

Curseofrust supports following networking protocols:

- `udp`: Fully compatible with curseofwar protocol.
- `tcp`
- `ws`: Work-in-progress WebSocket support.

## Arguments

The command line arguments are compatible with curseofwar format. Use `-h` to make the program display help information.

## Platforms

### `curseofrust-console`

TUI/CLI implementation. Supports multiplayer.

#### Controlling

The console version supports three controlling modes, as follow.

##### Keyboard

Use keyboard to control the game. Same as `curseofwar`.

- **HJKL** and **Arrow Keys** to control cursor.
- **Space** to toggle flag.
- **X** to unflag all tiles.
- **C** to unflag half of the tiles randomly.
- **R** or **V** to build and upgrade houses.
- **F** and **S** to control speed.
- **P** to pause the game.
- **Q** to quit the game.

##### Termux

A touchscreen keymap designed for playing with _Termux_.

- Tapping an unselected tile to control cursor position.
- Tapping the selected tile to toggle flag.
- **Down Key** to unflag all tiles.
- **ALT + Down Key** to unflag half of the tiles randomly.
- **HOME** or **Up Key** to build and upgrade houses.
- **PGUP** and **PGDN** to control speed.
- **END** to pause the game.
- **ESC** to quit the game.

##### Hybrid

_Keyboard_ mode with following features:

- Clicking an unselected tile to control cursor position.
- Clicking the selected tile to toggle flag.

### `curseofrust-gui-cocoa`

GUI implementation based on Cocoa for macOS. Currently does not support multiplayer. It is now hosted [here](https://codeberg.org/DM-Earth/curseofrust-gui-cocoa).

### `curseofrust-server`

The dedicated server implementation with a CLI interface.
