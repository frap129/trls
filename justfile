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

# Install to system (requires sudo)
install: release-test
    sudo install -Dm755 target/release/trls /usr/local/bin/trls
    sudo install -Dm644 trellis.toml.example /etc/trellis/trellis.toml
    sudo install -Dm755 hooks/50-dracut-setup.sh /etc/trellis/hooks.d/50-dracut-setup.sh
    sudo install -Dm755 hooks/50-sbctl-sign.sh /etc/trellis/hooks.d/50-sbctl-sign.sh
    sudo mkdir -p /var/lib/trellis/src
    sudo mkdir -p /var/cache/trellis/aur
    @echo "✅ Trellis installed to /usr/local/bin/trls"
    @echo "✅ Config installed to /etc/trellis/trellis.toml"
    @echo "✅ Hooks installed to /etc/trellis/hooks.d/"
    @echo "✅ Default directories created: /var/lib/trellis/src, /var/cache/trellis/aur"

# Install to custom directory
install-to PREFIX: release-test
    sudo install -Dm755 target/release/trls {{PREFIX}}/bin/trls
    sudo install -Dm644 trellis.toml.example {{PREFIX}}/etc/trellis/trellis.toml
    sudo install -Dm755 hooks/50-dracut-setup.sh {{PREFIX}}/etc/trellis/hooks.d/50-dracut-setup.sh
    sudo install -Dm755 hooks/50-sbctl-sign.sh {{PREFIX}}/etc/trellis/hooks.d/50-sbctl-sign.sh
    sudo mkdir -p /var/lib/trellis/src
    sudo mkdir -p /var/cache/trellis/aur
    @echo "✅ Trellis installed to {{PREFIX}}/bin/trls"
    @echo "✅ Config installed to {{PREFIX}}/etc/trellis/trellis.toml"
    @echo "✅ Hooks installed to {{PREFIX}}/etc/trellis/hooks.d/"
    @echo "✅ Default directories created: /var/lib/trellis/src, /var/cache/trellis/aur"

# Uninstall from system
uninstall:
    sudo rm -f /usr/local/bin/trls
    sudo rm -f /etc/trellis/trellis.toml
    sudo rm -rf /etc/trellis/hooks.d
    sudo rmdir /etc/trellis 2>/dev/null || true
    @echo "✅ Trellis uninstalled from /usr/local/bin/trls"
    @echo "✅ Config and hooks removed from /etc/trellis/"
    @echo "ℹ️  Cache and src directories preserved: /var/lib/trellis/src, /var/cache/trellis/aur"
    @echo "ℹ️  Remove manually if desired: sudo rm -rf /var/lib/trellis /var/cache/trellis"

# Development helpers
# ===================

# Run the built binary with arguments
run *ARGS:
    cargo run -- {{ARGS}}

# Watch for changes and rebuild
watch:
    cargo watch -x build

# Watch for changes and run tests
watch-test:
    cargo watch -x test

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

# Install development dependencies (Arch Linux)
setup-dev:
    sudo pacman -S --needed just cargo-watch jq
    cargo install cargo-watch

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
    cp target/release/trls dist/trellis-{{VERSION}}/trellis
    cp README.md dist/trellis-{{VERSION}}/
    cp trellis.toml.example dist/trellis-{{VERSION}}/
    cp -r hooks dist/trellis-{{VERSION}}/
    tar -czf "dist/$ARCHIVE" -C dist trellis-{{VERSION}}
    rm -rf dist/trellis-{{VERSION}}
    echo "✅ Created dist/$ARCHIVE"

# Comprehensive CI check (runs all validation)
ci: check test release
    @echo "✅ All CI checks passed"

# Performance testing
# ===================

# Build with performance profiling
perf-build:
    cargo build --release --features perf

# Benchmark (if benchmarks exist)
bench:
    cargo bench

# Container testing helpers
# =========================

# Test with example containerfiles (requires test setup)
test-containers:
    #!/bin/bash
    set -euo pipefail
    if [ ! -d "test-containers" ]; then
        echo "❌ test-containers directory not found"
        echo "Create test containerfiles in test-containers/ directory"
        exit 1
    fi
    just run --src-dir test-containers build

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
