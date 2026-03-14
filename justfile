default: install

# Build in debug mode
build:
    cd swift-bridge && swift build
    cargo build

# Build in release mode
release:
    cd swift-bridge && swift build -c release
    cargo build --release

# Run the app
run: build
    cp swift-bridge/.build/debug/nudge-bridge target/debug/
    cargo run

# Install to ~/.local/bin
install: release
    cp target/release/nudge ~/.local/bin/
    cp swift-bridge/.build/release/nudge-bridge ~/.local/bin/

# Uninstall from ~/.local/bin
uninstall:
    rm -f ~/.local/bin/nudge ~/.local/bin/nudge-bridge

# Remove build artifacts
clean:
    cargo clean
    cd swift-bridge && swift package clean
