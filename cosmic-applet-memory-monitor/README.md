# COSMIC Memory Monitor Applet

A panel applet for the COSMIC desktop that shows current RAM usage as a ring
(donut) chart with the percentage in the centre. It reads `/proc/meminfo` every
two seconds; clicking it opens a small popup with the usage percentage and
used / total GiB.

The chart is drawn with iced's `Canvas` (`RingChart` in `src/lib.rs`): a faint
full-circle track with an accent-coloured arc on top, proportional to
`mem_used / mem_total`.

## Build prerequisites

Rust is pinned to the toolchain in `../rust-toolchain.toml`. You also need the
native libraries the workspace links against:

```bash
sudo apt install -y clang libclang-dev libxkbcommon-dev libwayland-dev \
  libdbus-1-dev libpulse-dev libpipewire-0.3-dev libinput-dev pkg-config
```

`clang`/`libclang-dev` are required by the workspace's bindgen-based `-sys`
crates; without them the build fails with
`Unable to find libclang`.

## Quick test (standalone)

From the workspace root, run the applet as a floating window to eyeball the
ring without touching the panel:

```bash
cargo run -p cosmic-applet-memory-monitor
```

## Install just this applet (no sudo, leaves packaged applets untouched)

All applets compile into a single multicall binary (`cosmic-applets`) that
picks which applet to run from `argv[0]`. To install only this one into your
home directory, build the binary, then place three files under `~/.local`:

```bash
# Build the multicall binary (release)
cargo build --release -p cosmic-applets   # or: just build-release

# 1. Binary — symlink the multicall binary under this applet's name.
#    ~/.local/bin must be on the PATH that cosmic-panel inherits.
mkdir -p ~/.local/bin
ln -sf "$PWD/target/release/cosmic-applets" ~/.local/bin/cosmic-applet-memory-monitor

# 2. Desktop file — the generated copy with localized strings.
install -Dm644 target/xdgen/com.system76.CosmicAppletMemoryMonitor.desktop \
  ~/.local/share/applications/com.system76.CosmicAppletMemoryMonitor.desktop

# 3. Icon
install -Dm644 cosmic-applet-memory-monitor/data/icons/scalable/apps/com.system76.CosmicAppletMemoryMonitor-symbolic.svg \
  ~/.local/share/icons/hicolor/scalable/apps/com.system76.CosmicAppletMemoryMonitor-symbolic.svg
```

Then restart the panel and add the applet:

```bash
pkill cosmic-panel   # cosmic-session respawns it
```

Open **Settings → Desktop → Panel → Add applet** and choose **Memory Monitor**.

> If the applet fails to launch, the usual cause is `~/.local/bin` not being on
> the `PATH` that `cosmic-panel` inherits. Either ensure it is exported in your
> session environment, or install system-wide with `sudo just install` from the
> workspace root (note: that overwrites the distro's packaged applet files).

## Install system-wide (all applets)

From the workspace root:

```bash
just build-release
sudo just install
```

This installs the shared binary, then symlinks, desktop files, and icons for
**every** applet (including this one) into `/usr`. It overwrites your
distribution's packaged COSMIC applet files.
