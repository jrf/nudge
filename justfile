default: install

# Build in debug mode
build:
    cargo build

# Build in release mode
release:
    cargo build --release

# Run the app
run:
    cargo run

# Install to ~/.local/bin
install: release
    cp target/release/nudge ~/.local/bin/

# Uninstall from ~/.local/bin
uninstall:
    rm -f ~/.local/bin/nudge

# Remove build artifacts
clean:
    cargo clean
