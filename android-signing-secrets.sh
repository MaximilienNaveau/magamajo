#!/usr/bin/env bash
# Checks the APK signing key against the key F-Droid knows for
# com.magamajo.trampoline, then shows (or sets) the four GitHub secrets the
# trampoline release workflow signs with:
#   ANDROID_KEYSTORE_BASE64  ANDROID_KEYSTORE_PASS
#   ANDROID_KEY_ALIAS_NAME   ANDROID_KEY_ALIAS_PASS
#
# GitHub never shows a secret again, so this is how to make sure they're
# right: check the key, then store it again.
#
# The key and its passwords are kept in android-signing-key.tar.gz.gpg,
# encrypted to Maximilien's personal GPG key (B08357270308E49F,
# maximilien.naveau@gmail.com): only that key's owner can decrypt it.
#
# Usage:
#   ./android-signing-secrets.sh [--all] [--set]
#       uses the encrypted backup: no questions (except GPG's passphrase)
#   ./android-signing-secrets.sh --backup [<keystore>]
#       asks the passwords, checks them, and (re)writes the encrypted backup
#       from <keystore> (default: ../trampoline_releases/magamajo.keystore)
#   ./android-signing-secrets.sh <keystore> [--all] [--set]
#       same as the first form, from a keystore and typed passwords
#
#   --all  also magamajo's own secrets, KEYSTORE_P12 and CONFIG_YML
#          (fdroid/keystore.p12 and fdroid/config.yml, checked against the
#          repo fingerprint of the F-Droid URL)
#   --set  store them with `gh secret set` instead of printing them
#
# The APK key (the Unity release key) is NOT fdroid/keystore.p12: that one
# signs the F-Droid repo index, a different key.
#
# Not covered, since they only exist on GitHub/Cachix: GH_ACCESS_TOKEN
# (magamajo), FDROID_TRIGGER_TOKEN and CACHIX_AUTH_TOKEN (trampoline).
#
# Decrypted files only go to a temporary folder (in RAM when possible),
# deleted on exit. Passwords are never taken as arguments (they'd end up in
# the shell history). Printed values land in your terminal's scrollback:
# clear it afterwards.
set -euo pipefail

# The certificate SHA-256 of the APKs F-Droid serves (3.1.3, 3.2.0), the same
# value as FDROID_SIGNER_SHA256 in trampoline's release.yml.
APK_SHA256="89ce10f08ba6e5a4c51adb3e1477c530b07a733b6b68073d58c78f5ef88f3402"
# The F-Droid repo's own key: the ?fingerprint= of the repo URL (README.md).
REPO_SHA256="d04f9c306c0ca32cc89c5d4ea871916330be95c91fb08f2ad101a11cbfaac31a"
# The GPG key the backup is encrypted to.
GPG_RECIPIENT="B08357270308E49F"
TRAMPOLINE="MaximilienNaveau/trampoline"
MAGAMAJO="MaximilienNaveau/magamajo"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FDROID_DIR="$HERE/fdroid"
BACKUP="$HERE/android-signing-key.tar.gz.gpg"

usage() { sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 1; }
keystore="" all=false set=false backup=false
for arg in "$@"; do
    case "$arg" in
        --all) all=true ;;
        --set) set=true ;;
        --backup) backup=true ;;
        -*) usage ;;
        *) keystore="$arg" ;;
    esac
done
if $backup && [ -z "$keystore" ]; then
    keystore="$HERE/../trampoline_releases/magamajo.keystore"
fi
if [ -n "$keystore" ] && [ ! -f "$keystore" ]; then
    echo "❌ No keystore at $keystore" >&2
    echo
    usage
fi
if [ -z "$keystore" ] && [ ! -f "$BACKUP" ]; then
    echo "❌ No backup yet ($BACKUP): make it with --backup." >&2
    exit 1
fi

# keytool comes with any JDK; take one from nixpkgs if none is installed.
if ! command -v keytool >/dev/null; then
    exec nix shell nixpkgs#jdk17_headless --command "$0" "$@"
fi

# In RAM (tmpfs) when the system has one for this user.
tmp=$(mktemp -d -p "${XDG_RUNTIME_DIR:-/tmp}")
chmod 700 "$tmp"
trap 'rm -rf "$tmp"' EXIT

# The certificate's SHA-256, as apksigner/fdroid report it (lowercase, no
# colons). Passwords come in through the environment variables named
# store_pass_env/key_pass_env (`-storepass:env`), so they never show on
# keytool's command line (visible to `ps`).
cert_sha256() { # keystore alias
    keytool -exportcert -rfc -alias "$2" -keystore "$1" -storepass:env store_pass_env 2>/dev/null \
        | keytool -printcert 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f'
}
# Only reading the private key checks its password: copy it to a throwaway
# keystore.
key_password_works() { # keystore alias
    keytool -importkeystore -noprompt \
        -srckeystore "$1" -srcstorepass:env store_pass_env \
        -srcalias "$2" -srckeypass:env key_pass_env \
        -destkeystore "$tmp/check-$RANDOM.p12" -deststoretype pkcs12 \
        -deststorepass:env key_pass_env -destkeypass:env key_pass_env >/dev/null 2>&1
}

# --- The APK key (trampoline's secrets) --------------------------------------
if [ -z "$keystore" ]; then
    # From the encrypted backup: one file per value, exactly as typed.
    mkdir "$tmp/backup"
    if ! gpg --quiet --decrypt "$BACKUP" | tar -xz -C "$tmp/backup"; then
        echo "❌ Couldn't decrypt $BACKUP (needs GPG key $GPG_RECIPIENT)." >&2
        exit 1
    fi
    keystore="$tmp/backup/release.keystore"
    store_pass=$(cat "$tmp/backup/keystore_pass")
    alias=$(cat "$tmp/backup/key_alias")
    key_pass=$(cat "$tmp/backup/key_pass")
    echo "Decrypted $(basename "$BACKUP") (key \"$alias\")."
    export store_pass_env="$store_pass" key_pass_env="$key_pass"
else
    read -rsp "APK keystore password: " store_pass; echo
    export store_pass_env="$store_pass"
    mapfile -t aliases < <(keytool -list -keystore "$keystore" -storepass:env store_pass_env 2>/dev/null \
        | sed -n 's/^\([^,]*\), .*PrivateKeyEntry.*$/\1/p' || true)
    if [ ${#aliases[@]} -eq 0 ]; then
        echo "❌ Wrong keystore password, or no private key in $keystore." >&2
        exit 1
    fi
    echo "Keys in this keystore: ${aliases[*]}"
    read -rp "Key alias [${aliases[0]}]: " alias
    alias="${alias:-${aliases[0]}}"
    read -rsp "Key password (Enter if the same as the keystore's): " key_pass; echo
    key_pass="${key_pass:-$store_pass}"
    export key_pass_env="$key_pass"
fi

sha256=$(cert_sha256 "$keystore" "$alias")
if [ -z "$sha256" ]; then
    echo "❌ No key named \"$alias\" in $keystore." >&2
    exit 1
fi
if [ "$sha256" != "$APK_SHA256" ]; then
    echo "❌ \"$alias\" is $sha256," >&2
    echo "   not F-Droid's APK key $APK_SHA256: wrong keystore or alias." >&2
    exit 1
fi
echo "✅ Certificate matches F-Droid's APK key."
if ! key_password_works "$keystore" "$alias"; then
    echo "❌ Wrong key password for \"$alias\"." >&2
    exit 1
fi
echo "✅ Key password works."

if $backup; then
    mkdir "$tmp/new"
    cp "$keystore" "$tmp/new/release.keystore"
    printf '%s' "$store_pass" > "$tmp/new/keystore_pass"
    printf '%s' "$alias" > "$tmp/new/key_alias"
    printf '%s' "$key_pass" > "$tmp/new/key_pass"
    tar -cz -C "$tmp/new" . | gpg --quiet --yes --encrypt --recipient "$GPG_RECIPIENT" --output "$BACKUP"
    echo "✅ Wrote $(basename "$BACKUP"), encrypted to $GPG_RECIPIENT. Commit it:"
    echo "   git add $(basename "$BACKUP") && git commit -m 'Back up the APK signing key'"
    exit 0
fi

# --- The F-Droid repo key (magamajo's secrets) -------------------------------
if $all; then
    for file in keystore.p12 config.yml; do
        if [ ! -f "$FDROID_DIR/$file" ]; then
            echo "❌ $FDROID_DIR/$file is missing (restore it: cd fdroid && gpg -d fdroid-secrets.tar.gz.gpg | tar xz)." >&2
            exit 1
        fi
    done
    # fdroid reads its keystore's alias and passwords from config.yml.
    config_value() { sed -n "s/^$1: *//p" "$FDROID_DIR/config.yml" | sed "s/^['\"]//; s/['\"] *$//"; }
    store_pass_env=$(config_value keystorepass)
    key_pass_env=$(config_value keypass)
    repo_alias=$(config_value repo_keyalias)
    repo_sha256=$(cert_sha256 "$FDROID_DIR/keystore.p12" "$repo_alias")
    if [ "$repo_sha256" != "$REPO_SHA256" ]; then
        echo "❌ fdroid/keystore.p12 (\"$repo_alias\", with config.yml's password) is" >&2
        echo "   \"$repo_sha256\", not the repo's fingerprint $REPO_SHA256." >&2
        exit 1
    fi
    echo "✅ fdroid/keystore.p12 matches the F-Droid repo fingerprint."
    if ! key_password_works "$FDROID_DIR/keystore.p12" "$repo_alias"; then
        echo "❌ config.yml's keypass doesn't open \"$repo_alias\" in fdroid/keystore.p12." >&2
        exit 1
    fi
    echo "✅ config.yml's passwords open it."
fi
echo

# --- Print or store ----------------------------------------------------------
current_repo=""
put() { # repo name value
    if $set; then
        printf '%s' "$3" | gh secret set "$2" --repo "$1"
        echo "✅ $1: $2 set"
    else
        if [ "$1" != "$current_repo" ]; then
            current_repo="$1"
            echo "===== $1 → Settings → Secrets and variables → Actions → Repository secrets"
            echo
        fi
        echo "$2:"; echo "$3"; echo
    fi
}
put "$TRAMPOLINE" ANDROID_KEYSTORE_BASE64 "$(base64 -w0 "$keystore")"
put "$TRAMPOLINE" ANDROID_KEYSTORE_PASS "$store_pass"
put "$TRAMPOLINE" ANDROID_KEY_ALIAS_NAME "$alias"
put "$TRAMPOLINE" ANDROID_KEY_ALIAS_PASS "$key_pass"
if $all; then
    put "$MAGAMAJO" KEYSTORE_P12 "$(base64 -w0 "$FDROID_DIR/keystore.p12")"
    put "$MAGAMAJO" CONFIG_YML "$(base64 -w0 "$FDROID_DIR/config.yml")"
fi
