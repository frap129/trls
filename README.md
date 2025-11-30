# Trellis - Modular Arch-based bootc Image Builder

Trellis is a tool for building and using modular, arch-based bootc images. It provides a Rust-based CLI for creating immutable system images with atomic updates through bootc integration.

## What is bootc?

bootc enables container-native Linux systems with atomic updates. Instead of traditional package management, your entire system becomes an immutable container image that can be atomically updated, rolled back, and deployed consistently across environments.

## Features

- Modular Arch-based bootc image construction
- Seamless bootc upgrade integration with atomic updates
- Persistent Pacman and AUR package caches
- Flexible stage-based architecture (base, desktop, apps, etc.)
- TOML configuration with CLI overrides

## Use Cases

- Immutable Arch Linux desktop systems
- Server deployments with atomic updates
- Reproducible development environments
- Custom Arch-based distributions

## Installation

### From Source

```bash
git clone <repository-url>
cd trellis
just install
```

### Configuration

Create a configuration file at `/etc/trellis/trellis.toml`, see trellis.toml.example as a reference.

## Usage

### Commands

#### `build-builder`

Build the pacstrap container used by other commands:

```bash
trls build-builder
```

#### `build`

Build all requested rootfs stages:

```bash
trls build
```

#### `run`

Run a command in the latest rootfs container:

```bash
trls run -- /bin/bash
trls run -- systemctl status
```

#### `clean`

Remove unused container images:

```bash
trls clean
```

#### `update`

Build rootfs image and perform atomic bootc upgrade (primary workflow):

```bash
trls update
```

#### `image`

Generate bootable disk images from built containers:

```bash
trls image --output my-system.img
```

Available options:
- `--build`: Build the image before generation (uses config defaults + global flags)
- `--image`: Image tag to use (default: rootfs_tag:latest from config)
- `--output`: Output path for the generated image (default: ./bootable.img)
- `--filesystem`: Filesystem type (default: ext4)
- `--size`: Image size in GB (default: 20)
- `--root-password`: Root password to set in the generated image

##### Setting Root Password

You can set the root password in the generated image:

```bash
trls image --root-password "<your-secure-password>" --output my-system.img
```

> **Security Warning**: Passwords provided via command-line are visible in process lists and shell history. For production environments, it is strongly recommended to avoid passing passwords directly on the command line. You can use your shell's `read` command to provide the password without saving it to history:
>
> ```bash
> read -s -p "Enter root password: " ROOT_PASS && trls image --root-password "$ROOT_PASS" --output my-system.img
> ```

### Command Line Options

All configuration options can be overridden via command line:

```bash
# Override builder tag
trls --builder-tag my-builder build-builder

# Override rootfs stages
trls --rootfs-stages base,custom,final build

# Override stages directory
trls --stages-dir /path/to/my/containerfiles build

# Enable build cache
trls --podman-build-cache true build

# Override pacman cache location
trls --pacman-cache /custom/cache/path build

# Add extra mounts
trls --extra-mounts /host/path1,/host/path2 build

# Add extra build contexts
trls --extra-contexts mycontext=/path/to/context build
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

Trellis recursively searches for containerfiles throughout the entire source directory tree, starting from the configured source directory (default: `/var/lib/trellis/src`).

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
trls --rootfs-stages "multi:stage1,multi:stage2,single" build
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
trls build

# Build with custom stages
trls --rootfs-stages base,custom,final build
```

### bootc Workflow

```bash
# Build and deploy new system image
trls update

# Build custom configuration
trls --rootfs-stages base,desktop,apps build

# Test before deployment
trls run -- /bin/bash
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
