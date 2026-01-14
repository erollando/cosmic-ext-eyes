# COSMIC Ext Eyes

COSMIC panel applet for Pop!_OS COSMIC that draws a pair of cartoon eyes whose pupils follow the mouse cursor.

Release date: see `VERSION`. Changes: see `CHANGELOG.md`.

## Build

```sh
cargo build --release
```

Or with `just`:

```sh
just build
```

## Install (local user)

```sh
just install
```

Manual install:

```sh
mkdir -p ~/.local/bin ~/.local/share/applications
cp target/release/cosmic-ext-eyes ~/.local/bin/cosmic-ext-eyes
chmod 0755 ~/.local/bin/cosmic-ext-eyes
cp dist/com.xinia.CosmicAppletEyes.desktop ~/.local/share/applications/com.xinia.CosmicAppletEyes.desktop
update-desktop-database ~/.local/share/applications
```

Restart the panel so it picks up the new desktop entry:

```sh
pkill cosmic-panel
```

Then open **Panel → Add Applet** and add **Eyes**.
The applet ID / desktop filename is `com.xinia.CosmicAppletEyes`.

If the panel fails to launch the applet, make sure `~/.local/bin` is in the session `PATH` (the panel inherits its environment from the session).

## Notes

- On Wayland, global cursor tracking requires COSMIC’s privileged applet socket; otherwise pupils only follow while hovered.
- Global-to-local alignment self-calibrates on hover and persists in `~/.local/state/cosmic-ext-eyes/` (usually hover once per output/layout).

## Security / privacy

- No network access.
- No telemetry.
- No privileged APIs beyond what COSMIC already provides to panel applets.
