#!/usr/bin/env bash

# Ensure we have proper permisions
chown builder:builder /home/builder/aur -R

# Build requested packages
for pkg in "$@"; do
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
    pkgver="$(grep "pkgver=" PKGBUILD | rev | cut -d'=' -f1 | rev)"
    if find . -name "${pkg}-${pkgver}*.pkg.tar*" -print -quit | grep -q .; then
        echo "$pkg already built"
    else
        # Build package
        su builder -c "makepkg -fcCs --noconfirm --skippgpcheck"
    fi

    # Copy to staging directory
    cp /home/builder/aur/$pkg/${pkg}-${pkgver}*.pkg.tar* /aur
done

