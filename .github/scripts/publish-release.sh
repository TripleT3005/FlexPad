#!/usr/bin/env bash
set -euo pipefail

tag=$1
title=$2
version=$3
changelog=$4
latest=$5
shift 5

# Notes

notes="$RUNNER_TEMP/notes-$tag.md"
awk -v heading="## [$version]" '
  index($0, heading) == 1 { found = 1; next }
  found && /^## \[/ { exit }
  found { print }
' "$changelog" | sed '/./,$!d' > "$notes"
if [ ! -s "$notes" ]; then
  echo "::error::$changelog has no text for version $version"
  exit 1
fi
if [ -n "${EXTRA_NOTES:-}" ]; then
  printf '\n%s\n' "$EXTRA_NOTES" >> "$notes"
fi

# Release

prerelease=()
case "$version" in 0.*|*-*) prerelease=(--prerelease) ;; esac

gh release create "$tag" "$@" \
  --repo "$GITHUB_REPOSITORY" \
  --target "$GITHUB_SHA" \
  --title "$title" \
  --notes-file "$notes" \
  --latest="$latest" \
  "${prerelease[@]}"

# VirusTotal

if [ -z "${VT_API_KEY:-}" ]; then
  echo "::warning::The repository secret VT_API_KEY is not set, so $tag has no VirusTotal scan"
  exit 0
fi

{
  cat "$notes"
  echo
  echo "---"
  echo
  echo "### VirusTotal Scan Report"
  echo "Scanned on: $(date -u +%Y-%m-%d)"
  echo
  for file in "$@"; do
    case "$file" in *.sha256) continue ;; esac
    curl --fail --silent --show-error \
      --header "x-apikey: $VT_API_KEY" \
      --form "file=@$file" \
      https://www.virustotal.com/api/v3/files > /dev/null
    hash=$(cut -d ' ' -f 1 "$file.sha256")
    echo "- **$(basename "$file"):** [Scan Report](https://www.virustotal.com/gui/file/$hash)"
  done
} > "$notes.scanned"

gh release edit "$tag" --repo "$GITHUB_REPOSITORY" --notes-file "$notes.scanned"
