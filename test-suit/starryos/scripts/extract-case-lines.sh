#!/usr/bin/sh
# Print every line starting with "CASE " (serial / probe stdout), sorted for set-compare.
# Strips CR. For single-line probes prefer extract-case-line.sh.
# Usage: extract-case-lines.sh [file]
#        cat log | extract-case-lines.sh
set -eu
if [ "$#" -ge 1 ]; then
  grep '^CASE ' "$1" | tr -d '\r' | sort -u || true
else
  tr -d '\r' | grep '^CASE ' | sort -u || true
fi
