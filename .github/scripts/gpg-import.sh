#!/usr/bin/env bash
# Import the Flatpak signing key into a throwaway keyring.
# Reads the armoured key from $KEY and an optional passphrase from $PASSPHRASE;
# exports GNUPGHOME and the fingerprint.
set -euo pipefail

[ -n "${KEY:-}" ] || { echo "::error::FLATPAK_GPG_KEY secret is not set"; exit 1; }

GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
chmod 700 "$GNUPGHOME"

echo "allow-loopback-pinentry" > "${GNUPGHOME}/gpg-agent.conf"
echo "pinentry-mode loopback" > "${GNUPGHOME}/gpg.conf"
gpgconf --kill gpg-agent >/dev/null 2>&1 || true

echo "$KEY" | gpg --batch --passphrase "${PASSPHRASE:-}" --import

fingerprint="$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr/ { print $10; exit }')"
gpg --batch --yes --export "$fingerprint" > "${GNUPGHOME}/pub.gpg"

printf 'test' | gpg --batch --yes --pinentry-mode loopback \
  --passphrase "${PASSPHRASE:-}" --local-user "$fingerprint" \
  --detach-sign -o /dev/null - \
  || { echo "::error::the signing key cannot sign non-interactively; check FLATPAK_GPG_PASSPHRASE"; exit 1; }

echo "GNUPGHOME=${GNUPGHOME}" >> "${GITHUB_ENV:-/dev/stdout}"
echo "fingerprint=${fingerprint}" >> "${GITHUB_OUTPUT:-/dev/stdout}"
