# nxm-switcher

A small Linux CLI tool that switches the default handler for NXM links (`x-scheme-handler/nxm` and `x-scheme-handler/nxm-protocol`) between different mod managers using `xdg-mime`.

## Prerequisites

- **Linux** with an XDG-compatible desktop environment
- `xdg-mime` (part of `xdg-utils`, available in most distros)
- One or more mod managers installed with their `.desktop` files registered (see [Supported handlers](#supported-handlers))
- [Rust / Cargo](https://rustup.rs/) for building from source

## Installation

```bash
cargo install --git https://github.com/RyanLangston/nxm-switcher
```

Or clone and install locally:

```bash
git clone https://github.com/RyanLangston/nxm-switcher
cd nxm-switcher
cargo install --path .
```

## Usage

```
nxm <COMMAND>
```

### Commands

| Command | Description |
|---------|-------------|
| `nxm status` | Show the currently active NXM handler |
| `nxm select` | Interactively pick a handler from a menu |
| `nxm set <name>` | Non-interactively set a handler by name |
| `nxm --help` | Show help |
| `nxm --version` | Show version |

### Examples

```bash
# Show the active handler
nxm status
# Current handler: Mod Organizer 2 (modorganizer2-nxm-handler.desktop)

# Pick interactively
nxm select
# Select NXM handler
# > Vortex
#   Mod Organizer 2
#   Nexus Mods App

# Set directly (useful in scripts or dotfiles, case-insensitive)
nxm set "Mod Organizer 2"
nxm set vortex
```

## Configuration

On first run of `nxm select` or `nxm set`, a default config file is created at:

```
~/.config/nxm/config.toml
```

### Config format

```toml
handlers = [
  { name = "Vortex", desktop = "vortex-steamtinkerlaunch-dl.desktop" },
  { name = "Mod Organizer 2", desktop = "modorganizer2-nxm-handler.desktop" },
  { name = "Nexus Mods App", desktop = "com.nexusmods.app.desktop" },
]
```

- **`name`** — Human-readable label shown in the menu and in `status` output.
- **`desktop`** — The `.desktop` filename that `xdg-mime` will register as the handler. The file must exist in `~/.local/share/applications/` or `/usr/share/applications/`.

Add, remove, or rename entries to match the mod managers you have installed.

## Supported handlers

The default config ships with entries for:

| Name | Desktop file | Typical source |
|------|-------------|----------------|
| Vortex | `vortex-steamtinkerlaunch-dl.desktop` | [SteamTinkerLaunch](https://github.com/sonic2kk/steamtinkerlaunch) |
| Mod Organizer 2 | `modorganizer2-nxm-handler.desktop` | [Mod Organizer 2](https://github.com/ModOrganizer2/modorganizer) |
| Nexus Mods App | `com.nexusmods.app.desktop` | [Nexus Mods App](https://www.nexusmods.com/about/app/) |

If a configured `.desktop` file is not found on disk, `nxm` will print a warning but still continue.
