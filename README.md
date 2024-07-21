# curseofrust

Curseofrust is [curseofwar](https://github.com/a-nikolaev/curseofwar) (Real Time Strategy Game for Linux) re-implemented in Rust.

The game supports both singleplayer and multiplayer, along with different platforms and networking protocols.

## Platforms

- `curseofrust-console`: TUI/CLI implementation. Supports multiplayer.
- `curseofrust-gui-cocoa`: GUI implementation based on Cocoa for macOS. Currently does not support multiplayer.
- `curseofrust-server`: The dedicated server implementation with a CLI interface.

## Protocols

Curseofrust supports following networking protocols:

- `udp`: Fully compatible with curseofwar protocol.
- `tcp`
- `ws`: The WebSocket protocol. Currently not useable.

## Arguments

The command line arguments are compatible with curseofwar format. Use `-h` to make the program display help information.