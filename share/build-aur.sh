#!/usr/bin/env bash

# Ensure we have proper permisions
chown builder:builder /home/builder/aur -R

# Ensure package databases are up to date
pacman -Sy

# Parse input
pkgs=()
args="-fcCs --noconfirm --skippgpcheck"
while test $# -gt 0; do
  case "$1" in
    -*)
      args="$args $1"
      shift
      ;;
    *)
      pkgs+=("$1")
      shift
      ;;
  esac
done

# Build requested packages
for pkg in "${pkgs[@]}"; do
    if [[ ! -d "/home/builder/aur/$pkg" ]]; then
        # Get sources
        su builder -c "git clone https://aur.archlinux.org/$pkg.git /home/builder/aur/$pkg"
        cd /home/builder/aur/$pkg
    else
        # Update existing sources
        cd /home/builder/aur/$pkg
        su builder -c "git pull"
    fi

    # Check if package exists in cache before building
    need_build=false
    for filename in "$(su builder -c 'makepkg --packagelist')"; do
        filename="$(echo $filename | rev | cut -d/ -f1 | rev)"
        if [[ -f "$filename" ]]; then
            echo "$filename already built"
        else
            need_build=true
            rm *.pkg.tar.zst
            break
        fi
    done

    # Build package
    ($need_build) && su builder -c "makepkg $args"

    # Copy to staging directory
    cp /home/builder/aur/$pkg/${pkg}-*.pkg.* /aur
done
 
