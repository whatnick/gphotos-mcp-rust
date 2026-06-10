#!/usr/bin/env bash
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  exit 0
fi

staged_files=$(git diff --cached --name-only --diff-filter=ACMR || true)

blocked_path_regex='(^|/)(\.env(\..*)?|tokens\.json|.*\.credentials\.json|.*\.secret|.*\.pem|\.credentials(/|$)|\.secrets(/|$))'
blocked_content_regex='(GOCSPX-[A-Za-z0-9_-]+|AIza[0-9A-Za-z_-]{35}|-----BEGIN (RSA )?PRIVATE KEY-----|AKIA[0-9A-Z]{16})'

while IFS= read -r file; do
  [ -n "$file" ] || continue

  if printf '%s\n' "$file" | grep -Eq "$blocked_path_regex"; then
    echo "Refusing staged secret-bearing path: $file" >&2
    exit 1
  fi

  if [ ! -f "$file" ]; then
    continue
  fi

  if git diff --cached --unified=0 --no-color -- "$file" | grep -Eq "$blocked_content_regex"; then
    echo "Refusing staged secret-like content in: $file" >&2
    exit 1
  fi
done <<< "$staged_files"

exit 0
