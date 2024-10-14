#!/usr/bin/env bash

# Build requested packages
for pkg in "$@"; do
    su builder -c "git clone https://aur.archlinux.org/$pkg.git /home/builder/aur/$pkg"
    cd /home/builder/aur/$pkg
    su builder -c "makepkg -s --noconfirm --skippgpcheck"
    cp /home/builder/aur/$pkg/${pkg}*.tar.zst /aur
done

