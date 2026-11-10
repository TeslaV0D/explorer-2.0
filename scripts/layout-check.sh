#!/bin/bash
# Layout check (localisation L5): reads the overflow lists of one or more
# galleries (scripts/gallery.sh writes <dir>/overflow.txt from the UI dump
# with EXPLORER2_UI_OVERFLOW=1) and fails if any text does not fit:
#   w = one line cut off, h = wrapped lines cut off,
#   c = sticks out of a clipping parent (panel, scroll area, popup),
#   p = sticks out of the element right around it.
# Elided texts (“…”, e) are on purpose (file names, paths, column headers)
# and only listed. The developer UI kit (12-ui-kit) is not part of the product.
#   scripts/layout-check.sh <gallery-dir>...
set -u
FAIL=0
for dir in "$@"; do
  f=$dir/overflow.txt
  if [ ! -f "$f" ]; then echo "FAIL $dir: no overflow.txt (gallery did not run?)"; FAIL=1; continue; fi
  shots=$(wc -l < "$dir/index.txt")
  bad=$(awk -F'\t' '$1 !~ /-12-ui-kit$/ && $7 !~ /^e/' "$f")
  elided=$(awk -F'\t' '$1 !~ /-12-ui-kit$/ && $7 ~ /^e/ { print $2 }' "$f" | sort | uniq -c | sort -rn)
  if [ -n "$bad" ]; then
    echo "FAIL $dir: texts that do not fit ($shots screens):"
    printf '%s\n' "$bad" | awk -F'\t' '{ printf "  %-28s %-6s %s\n", $1, $7, $2 }'
    FAIL=1
  else
    echo "PASS $dir: every text fits ($shots screens)"
  fi
  [ -n "$elided" ] && { echo "  elided on purpose (count, text):"; printf '%s\n' "$elided" | sed 's/^/   /'; }
done
exit $FAIL
