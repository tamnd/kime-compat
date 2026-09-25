#!/usr/bin/env bash
# Runs Laya's own test suite, from the pinned Laya sdist, against the kime package through the
# shim in laya-suite/shim, and compares each file's outcome with laya-suite/expected.tsv. A file
# that passes when it should fail is reported too, so the list cannot go stale.
#
#   ci/laya-suite.sh <python with kime installed> [models dir]
#
# The models dir is where the laya checkpoint is, for the tests that load it. KIME_IDENTIFIER=0
# is set for the run, so routing is Laya's word lists alone and Laya's routing tests apply as
# written. kime's own routing on top of that is tested in tamnd/kime.
set -euo pipefail

py="$1"
models="${2:-$HOME/data/kime/models}"
here="$(cd "$(dirname "$0")/.." && pwd)"
version=0.3.20
url=https://files.pythonhosted.org/packages/31/82/0964e13e1a67ae4ac2824c3115470d31cd149a065d971a5dec6e4049ca23/laya-$version.tar.gz
sha=692de1346cf0239bb7bbcffdf0cfbd538c24834f8bc61e9b60f4106465c033c9

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl -sSfL -o "$work/laya.tar.gz" "$url"
echo "$sha  $work/laya.tar.gz" | shasum -a 256 -c - > /dev/null
tar -xzf "$work/laya.tar.gz" -C "$work"
mkdir "$work/run"
cp -R "$work/laya-$version/tests" "$work/run/tests"
# LAYA_SUITE_SHIM=0 leaves the shim out, so the installed Laya runs its own tests as a baseline.
[ "${LAYA_SUITE_SHIM:-1}" = 0 ] || cp -R "$here/laya-suite/shim/laya" "$work/run/laya"
mkdir "$work/logs"

export KIME_IDENTIFIER=0 LAYA_DEVICE=cpu HF_HUB_OFFLINE=1
export KIME_MODELS="$models" LAYA_MODELS="$models" LAYA_TEST_MODEL="$models/laya"

# One file: pass, skip or fail. A file runs as a script, then under pytest when it has test
# functions. A script that prints SKIP, or stops on pytest.importorskip, is a skip.
outcome() {
  local f="$1" log="$2"
  if ! (cd "$work/run" && PYTHONPATH="$work/run" timeout 600 "$py" "$f") > "$log" 2>&1; then
    # pytest.importorskip at the top of a file raises Skipped when it runs as a script.
    if grep -q "^Skipped: " "$log"; then echo skip; else echo fail; fi
    return
  fi
  if grep -q "^SKIP" "$log"; then
    echo skip
    return
  fi
  if grep -q "^def test_\|^    def test_" "$work/run/$f"; then
    local code=0
    (cd "$work/run" && PYTHONPATH="$work/run" timeout 600 "$py" -m pytest -q -p no:cacheprovider "$f") >> "$log" 2>&1 || code=$?
    if [ "$code" = 5 ] || { [ "$code" = 0 ] && grep -q " skipped" "$log" && ! grep -q " passed" "$log"; }; then
      echo skip
      return
    fi
    [ "$code" = 0 ] || { echo fail; return; }
  fi
  echo pass
}

bad=0
pass=0
total=0
while IFS=$'\t' read -r file want reason; do
  case "$file" in "#"* | file) continue ;; esac
  total=$((total + 1))
  log="$work/logs/$file.log"
  got="$(outcome "tests/$file" "$log")"
  [ "$got" = pass ] && pass=$((pass + 1))
  if [ "$got" = "$want" ]; then
    printf '%-34s %-5s %s\n' "$file" "$got" "$reason"
  else
    printf '%-34s %-5s want %s: %s\n' "$file" "$got" "$want" "$reason"
    tail -20 "$log"
    bad=$((bad + 1))
  fi
done < "$here/laya-suite/expected.tsv"

for f in "$work/run/tests"/test_*.py; do
  if ! grep -q "^$(basename "$f")	" "$here/laya-suite/expected.tsv"; then
    echo "$(basename "$f") is not in laya-suite/expected.tsv"
    bad=$((bad + 1))
  fi
done

echo "$pass of $total files pass, $bad not as expected"
[ "$bad" = 0 ]
