#!/usr/bin/env bash
# Import the Flatpak signing key into a throwaway keyring.
# Reads the armoured key from $KEY; exports GNUPGHOME and the fingerprint.
set -euo pipefail

[ -n "${KEY:-}" ] || { echo "::error::FLATPAK_GPG_KEY secret is not set"; exit 1; }

GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
echo "$KEY" | gpg --batch --import

fingerprint="$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr/ { print $10; exit }')"
gpg --batch --yes --export "$fingerprint" > "${GNUPGHOME}/pub.gpg"

echo "GNUPGHOME=${GNUPGHOME}" >> "${GITHUB_ENV:-/dev/stdout}"
echo "fingerprint=${fingerprint}" >> "${GITHUB_OUTPUT:-/dev/stdout}"
