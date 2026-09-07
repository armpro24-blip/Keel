#!/usr/bin/env bash
# Create the L1 repository from the seed at TARGET_DIR (outside Keel), with a
# fixed author and date so the seed commit hash is reproducible when the tree
# is byte-identical. Prints the commit and tree hashes, then runs the seed's
# own test suite, which must pass before the workload starts.
set -euo pipefail

seed="$(cd "$(dirname "$0")/seed" && pwd)"
target="${1:?usage: init_l1.sh TARGET_DIR}"
if [ -e "$target" ]; then
    echo "refusing to touch an existing path: $target" >&2
    exit 1
fi

mkdir -p "$target"
cp -R "$seed"/. "$target"/
cd "$target"
git -c core.autocrlf=false init -q -b main
git -c core.autocrlf=false add -A
GIT_AUTHOR_NAME="L1 seed" GIT_AUTHOR_EMAIL="l1@keel.invalid" \
GIT_COMMITTER_NAME="L1 seed" GIT_COMMITTER_EMAIL="l1@keel.invalid" \
GIT_AUTHOR_DATE="2026-09-07T00:00:00+00:00" GIT_COMMITTER_DATE="2026-09-07T00:00:00+00:00" \
git -c core.autocrlf=false commit -q -m "L1 seed: tally 0.1"

echo "commit $(git rev-parse HEAD)"
echo "tree   $(git rev-parse 'HEAD^{tree}')"
python -m unittest discover -s tests
