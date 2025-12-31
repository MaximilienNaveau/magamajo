#!/bin/bash
set -e

# Backup F-Droid secrets to encrypted tarball
# Usage: ./backup-secrets.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Files to backup
SECRET_FILES=("config.yml" "keystore.p12")

# Check if files exist
for file in "${SECRET_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "Error: $file not found!"
        exit 1
    fi
done

# Create tarball
TARBALL="fdroid-secrets.tar.gz"
ENCRYPTED="fdroid-secrets.tar.gz.gpg"

echo "Creating tarball..."
tar czf "$TARBALL" "${SECRET_FILES[@]}"

echo "Encrypting with GPG..."
echo "You will be prompted for a passphrase. REMEMBER IT!"
gpg --symmetric --cipher-algo AES256 "$TARBALL"

# Remove unencrypted tarball
rm "$TARBALL"

echo ""
echo "✅ Encrypted backup created: $ENCRYPTED"
echo "📝 Remember your passphrase!"
echo ""
echo "To decrypt later:"
echo "  gpg -d $ENCRYPTED | tar xz"
echo ""
echo "Now commit the encrypted file:"
echo "  git add $ENCRYPTED"
echo "  git commit -m 'Update F-Droid secrets backup'"
echo "  git push"
