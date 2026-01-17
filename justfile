build:
    cargo build --release

install: build
    # Install the binary somewhere in your session PATH.
    mkdir -p ~/.local/bin
    cp target/release/cosmic-ext-eyes ~/.local/bin/cosmic-ext-eyes
    chmod 0755 ~/.local/bin/cosmic-ext-eyes

    # Install icons (scalable + symbolic).
    mkdir -p ~/.local/share/icons/hicolor/scalable/apps
    cp dist/com.xinia.CosmicAppletEyes.svg ~/.local/share/icons/hicolor/scalable/apps/com.xinia.CosmicAppletEyes.svg
    cp dist/com.xinia.CosmicAppletEyes-symbolic.svg ~/.local/share/icons/hicolor/scalable/apps/com.xinia.CosmicAppletEyes-symbolic.svg
    gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor >/dev/null 2>&1 || true

    # Ensure the desktop entry directory exists
    mkdir -p ~/.local/share/applications

    # Copy the desktop entry file
    cp dist/com.xinia.CosmicAppletEyes.desktop ~/.local/share/applications/com.xinia.CosmicAppletEyes.desktop

    # Update desktop database
    update-desktop-database ~/.local/share/applications

    echo "Applet installed. You may need to restart the COSMIC panel or session."
    echo "To restart the panel:"
    echo "pkill cosmic-panel"
    echo "or:"
    echo "systemctl --user restart cosmic-panel.service"

run: build
    cargo run --release

clean:
    cargo clean
