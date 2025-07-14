# Trellis - Container Build System

A Rust-based CLI utility for managing multi-stage bootc container builds with Podman. This is a port of the original bash script to Rust for better performance, error handling, and maintainability.

## Features

- Multi-stage container builds
- Persistent package caches for Pacman and AUR
- Flexible configuration via TOML file and command-line arguments
- Support for custom build contexts and mounts
- Integration with bootc for system updates

## Installation

### From Source

```bash
git clone <repository-url>
cd trellis
cargo build --release
sudo cp target/release/trellis /usr/local/bin/
```

### Configuration

Create a configuration file at `/etc/trellis/trellis.toml`, see trells.toml.example as a reference.

## Usage

### Commands

#### `build-builder`

Build the pacstrap container used by other commands:

```bash
trellis build-builder
```

#### `build`

Build all requested rootfs stages:

```bash
trellis build
```

#### `run`

Run a command in the latest rootfs container:

```bash
trellis run -- /bin/bash
trellis run -- systemctl status
```

#### `clean`

Remove unused container images:

```bash
trellis clean
```

#### `update`

Shorthand to build rootfs and run bootc upgrade:

```bash
trellis update
```

### Command Line Options

All configuration options can be overridden via command line:

```bash
# Override builder tag
trellis --builder-tag my-builder build-builder

# Override rootfs stages
trellis --rootfs-stages base,custom,final build

# Override source directory
trellis --src-dir /path/to/my/containerfiles build

# Enable build cache
trellis --podman-build-cache true build

# Override pacman cache location
trellis --pacman-cache /custom/cache/path build

# Add extra mounts
trellis --extra-mounts /host/path1,/host/path2 build

# Add extra build contexts
trellis --extra-contexts mycontext=/path/to/context build
```

### Directory Structure

The tool supports two ways to organize Containerfiles in the source directory:

#### 1. Flat Structure (Legacy)

Containerfiles in the root source directory:

```
src/
├── Containerfile.base      # Base stage
├── Containerfile.tools     # Tools stage
├── Containerfile.system    # System stage
└── Containerfile.apps      # Apps stage
```

#### 2. Nested Structure (Recommended)

Containerfiles organized in subdirectories by group name:

```
src/
├── base/
│   └── Containerfile.base
├── builder/
│   └── Containerfile.builder
├── features/
│   ├── gpu/
│   │   └── Containerfile.gpu
│   ├── bluetooth/
│   │   └── Containerfile.bluetooth
│   └── interactive/
│       └── Containerfile.interactive
├── desktops/
│   ├── hyprland/
│   │   └── Containerfile.hyprland
│   └── cosmic/
│       └── Containerfile.cosmic
└── finalize/
    └── Containerfile.finalize
```

#### Containerfile Discovery

Trellis recursively searches for containerfiles throughout the entire source directory tree, starting from the configured source directory (default: `src/`).

When building a stage named `{group}`, trellis will search for `Containerfile.{group}` in:
- The root source directory: `src/Containerfile.{group}`
- Any subdirectory: `src/{path}/Containerfile.{group}`

For example, when building stage `gpu`, trellis will find the containerfile whether it's located at:
- `src/Containerfile.gpu` (flat structure)
- `src/gpu/Containerfile.gpu` (nested structure)
- `src/features/gpu/Containerfile.gpu` (deeply nested structure)

#### Multi-stage Builds

For multi-stage Containerfiles, use the format `<group>:<stage>`:

```bash
trellis --rootfs-stages "multi:stage1,multi:stage2,single" build
```

This will look for:

- `Containerfile.multi` with stages `stage1` and `stage2`
- `Containerfile.single` with a single stage

### Build Arguments

The tool automatically passes build arguments:

- `BASE_IMAGE`: Set to the previous stage's image
- `HOOKS_DIR`: Set to `/etc/trellis/hooks.d` if it exists

### Caching

The tool supports persistent caching:

- **Pacman cache**: Persistent package cache for faster builds
- **AUR cache**: Persistent AUR package build cache
- **Podman build cache**: Can be enabled/disabled via configuration

### Hooks

Place executable scripts in `/etc/trellis/hooks.d/` to run custom logic during builds.

## Examples

### Basic Build

```bash
# Build with default configuration
trellis build

# Build with custom stages
trellis --rootfs-stages base,custom,final build
```

### Development Workflow

```bash
# Build builder container
trellis build-builder

# Build development rootfs
trellis --rootfs-stages base,devel build

# Test the build
trellis run -- /bin/bash

# Clean up
trellis clean
```

## Error Handling

The tool provides detailed error messages and proper exit codes:

- Exit code 0: Success
- Exit code 1: Error occurred

Error messages are prefixed with `====> ERROR:` for easy identification.

## Dependencies

- Rust 1.70+
- Podman
- bootc (for update command)

## License

MIT License - see LICENSE file for details.
