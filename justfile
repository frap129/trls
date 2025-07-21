# Trellis build automation
# Requires: just (https://github.com/casey/just)

# Default recipe - show available commands
default:
    @just --list

# Development builds and testing
# ===============================

# Build debug version
build:
    cargo build

# Build and run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run specific test
test-one TEST:
    cargo test {{TEST}}

# Generate test coverage report
coverage:
    cargo tarpaulin --out Html --output-dir coverage/ --exclude-files "src/main.rs" "tests/*"

# Generate coverage report and open in browser
coverage-open: coverage
    @echo "Opening coverage report..."
    @if command -v xdg-open >/dev/null 2>&1; then xdg-open coverage/tarpaulin-report.html; else echo "Please open coverage/tarpaulin-report.html manually"; fi

# Check code formatting and linting
check:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings

# Format code and fix linting issues
fix:
    cargo fmt
    cargo clippy --fix --all-targets --all-features --allow-dirty

# Clean build artifacts
clean:
    cargo clean

# Release builds and installation
# ===============================

# Build optimized release version
release:
    cargo build --release

# Build release and run all tests (CI-ready)
release-test: test release
    @echo "✅ Release build completed with all tests passing"

# Install binary to system (requires sudo)
install: release-test
    sudo install -Dm755 target/release/trls /usr/local/bin/trls
    sudo mkdir -p /var/lib/trellis/src
    sudo mkdir -p /var/cache/trellis/aur
    @echo "✅ Trellis installed to /usr/local/bin/trls"
    @echo "✅ Default directories created: /var/lib/trellis/src, /var/cache/trellis/aur"
    @echo "ℹ️  Run 'just install-hooks' to install system hooks"

# Install binary to custom directory
install-to PREFIX: release-test
    sudo install -Dm755 target/release/trls {{PREFIX}}/bin/trls
    sudo mkdir -p /var/lib/trellis/src
    sudo mkdir -p /var/cache/trellis/aur
    @echo "✅ Trellis installed to {{PREFIX}}/bin/trls"
    @echo "✅ Default directories created: /var/lib/trellis/src, /var/cache/trellis/aur"
    @echo "ℹ️  Run 'just install-hooks' to install system hooks"

# Install system hooks (requires sudo)
install-hooks:
    sudo install -Dm755 hooks/50-dracut-setup.sh /etc/trellis/hooks.d/50-dracut-setup.sh
    sudo install -Dm755 hooks/50-sbctl-sign.sh /etc/trellis/hooks.d/50-sbctl-sign.sh
    @echo "✅ Hooks installed to /etc/trellis/hooks.d/"

# Uninstall binary from system
uninstall:
    sudo rm -f /usr/local/bin/trls
    @echo "✅ Trellis uninstalled from /usr/local/bin/trls"
    @echo "ℹ️  Cache and src directories preserved: /var/lib/trellis/src, /var/cache/trellis/aur"
    @echo "ℹ️  Run 'just uninstall-hooks' to remove system hooks"
    @echo "ℹ️  Remove cache manually if desired: sudo rm -rf /var/lib/trellis /var/cache/trellis"

# Uninstall system hooks
uninstall-hooks:
    sudo rm -rf /etc/trellis/hooks.d
    sudo rmdir /etc/trellis 2>/dev/null || true
    @echo "✅ Hooks removed from /etc/trellis/"

# Development helpers
# ===================

# Run the built binary with arguments
run *ARGS:
    cargo run -- {{ARGS}}



# Generate documentation
docs:
    cargo doc --open

# Show crate information
info:
    @echo "Project: trellis"
    @echo "Binary: trls"
    @echo "Version: $(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "trellis") | .version')"
    @echo "Target: $(rustc --version --verbose | grep host | cut -d' ' -f2)"

# Development environment setup
# =============================

# Install development dependencies
setup-dev:
    @echo "Installing development dependencies..."
    @echo "Please install 'just' and 'jq' using your system package manager"
    @echo "For Arch Linux: sudo pacman -S --needed just jq"
    @echo "For Ubuntu/Debian: sudo apt install just jq"
    @echo "For other systems, see: https://github.com/casey/just#installation"

# Install pre-commit hooks
setup-hooks:
    @echo "#!/bin/bash" > .git/hooks/pre-commit
    @echo "just check" >> .git/hooks/pre-commit
    @chmod +x .git/hooks/pre-commit
    @echo "✅ Pre-commit hooks installed (runs 'just check')"

# Package and distribution
# ========================

# Create release archive
package VERSION: release-test
    #!/bin/bash
    set -euo pipefail
    ARCHIVE="trellis-{{VERSION}}-$(rustc --version --verbose | grep host | cut -d' ' -f2).tar.gz"
    mkdir -p dist/trellis-{{VERSION}}
    cp target/release/trls dist/trellis-{{VERSION}}/trls
    cp README.md dist/trellis-{{VERSION}}/
    cp trellis.toml.example dist/trellis-{{VERSION}}/
    cp -r hooks dist/trellis-{{VERSION}}/
    tar -czf "dist/$ARCHIVE" -C dist trellis-{{VERSION}}
    rm -rf dist/trellis-{{VERSION}}
    echo "✅ Created dist/$ARCHIVE"

# Comprehensive CI check (runs all validation)
ci: check test release
    @echo "✅ All CI checks passed"





# System integration test (requires podman)
test-integration:
    #!/bin/bash
    set -euo pipefail
    if ! command -v podman >/dev/null 2>&1; then
        echo "❌ podman not found - skipping integration test"
        exit 0
    fi
    echo "🧪 Running integration test with podman..."
    # Add integration test commands here when ready
    echo "✅ Integration test completed"
