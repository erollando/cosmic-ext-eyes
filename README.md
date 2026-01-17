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
mkdir -p ~/.local/share/icons/hicolor/scalable/apps
cp dist/com.xinia.CosmicAppletEyes.svg ~/.local/share/icons/hicolor/scalable/apps/com.xinia.CosmicAppletEyes.svg
cp dist/com.xinia.CosmicAppletEyes-symbolic.svg ~/.local/share/icons/hicolor/scalable/apps/com.xinia.CosmicAppletEyes-symbolic.svg
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor >/dev/null 2>&1 || true
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
- Global-to-local alignment self-calibrates on hover and persists in `~/.local/state/cosmic-ext-eyes/` (usually hover once per output + applet instance).
- Offset file naming: `offset-<output>-<instance>.txt`. You can override the `<instance>` part by setting `COSMIC_EYES_OFFSET_KEY`.

## Security / privacy

- No network access.
- No telemetry.
- No privileged APIs beyond what COSMIC already provides to panel applets.
