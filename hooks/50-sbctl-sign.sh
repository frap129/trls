#!/usr/bin/env bash

# Only run if sbctl is installed
[[ $(command -v sbctl) ]] || return

# Create secureboot keys if they don't exist
sbctl create-keys --quiet

# Sign vmlinuz and all .efi files
vmlinuz=$(find /usr/lib/modules -name vmlinuz)
readarray -d '' files_to_sign < <(find / -name "*.efi" -print0)
files_to_sign+=("$vmlinuz")
for file in "${files_to_sign[@]}"; do
  sbctl sign "${file}"
done
