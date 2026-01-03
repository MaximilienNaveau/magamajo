#!/bin/bash

cd metascoop
echo "::group::Building metascoop-rs executable"
nix develop --command bash -c "cargo build --release"
echo "::endgroup::"
cd ..

# Run metascoop from parent directory inside nix develop so fdroid is available
nix develop --command bash -c "./metascoop/target/release/metascoop --apps-path=apps.yaml --repo-dir=fdroid/repo --personal-access-token=\"$GH_ACCESS_TOKEN\" $1"
EXIT_CODE=$?

echo "Scoop had an exit code of $EXIT_CODE"

set -e

if [ $EXIT_CODE -eq 2 ]; then
    # Exit code 2 means that there were no significant changes
    echo "This means that there were no significant changes"
    exit 0
elif [ $EXIT_CODE -eq 0 ]; then
    # Exit code 0 means that we can commit everything & push

    echo "This means that we now have changes we should push"
else 
    echo "This is an unexpected error"

    exit $EXIT_CODE
fi
