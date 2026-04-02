# vp

A CLI tool that converts Minecraft server files between Paper MC and Vanilla formats. It auto-detects the source format and converts in the appropriate direction.

When converting Paper to Vanilla, Paper-specific files (bukkit.yml, spigot.yml, config/, plugins/, etc.) are preserved in a `papercfg.old` folder in the output directory.

## Install

Requires Rust. Install with cargo:

```
cargo install --path .
```

## Usage

```
vp <source> <dest>
```

- `source` - path to the existing Minecraft server directory
- `dest` - path to the output directory (created if it doesn't exist)

Example:

```
vp ./my-paper-server ./vanilla-server
```
