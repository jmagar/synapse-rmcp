#!/usr/bin/env bash
# =============================================================================
# check-rust-module-size.sh — NO MONOLITHS gate for Rust production modules
#
# Ported from syslog-mcp and adapted for synapse2. Counts non-comment /
# non-blank / non-doc lines (real code) per production .rs file. Blank lines,
# line comments (// /// //!), and block comments (/* ... */) are NOT counted.
# Test files are exempt: *_tests.rs, *test.rs, anything under tests/.
#
# TWO-TIER policy (line count is a proxy; cohesion is the real goal):
#   - SOFT (default 400): advisory. Prints a "check cohesion / consider
#     splitting" notice but does NOT fail. Crossing 400 is a prompt to look at
#     the module, not an automatic split mandate.
#   - HARD (default 1000): a true monolith. FAILS (exit 1) — split it.
#
# Usage:
#   scripts/check-rust-module-size.sh [--soft N] [--hard N] [--self-test] [PATH ...]
#     --limit N   alias for --soft N (back-compat / ad-hoc "what exceeds N" queries)
#     (no PATH)   checks every tracked + untracked .rs file (CI / `just`)
#     (PATH ...)  checks only those files/dirs (lefthook staged files)
# =============================================================================
set -euo pipefail

soft=400
hard=1000
self_test=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --soft|--limit)
      soft="${2:?$1 requires a value}"
      shift 2
      ;;
    --hard)
      hard="${2:?--hard requires a value}"
      shift 2
      ;;
    --self-test)
      self_test=1
      shift
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: scripts/check-rust-module-size.sh [--soft N] [--hard N] [--self-test] [PATH ...]

Reports non-test Rust production files by real-code line count.
  > soft (default 400): advisory notice, does NOT fail.
  > hard (default 1000): monolith, fails with exit 1.
Blank lines, line/doc comments, and block comments are ignored.
When PATH values are provided, only those files/directories are checked.
USAGE
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "unknown option: $1" >&2
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

count_file() {
  perl -0ne '
    s{/\*.*?\*/}{}gs;
    my $count = 0;
    for my $line (split /\n/) {
      $line =~ s/^\s+//;
      $line =~ s/\s+$//;
      next if $line eq "";
      next if $line =~ m{^//};
      $count++;
    }
    print "$count\n";
  ' "$1"
}

is_prod_rust_file() {
  local file="$1"
  [[ "$file" == *.rs ]] || return 1
  [[ "$file" != *_tests.rs ]] || return 1
  [[ "$file" != *test.rs ]] || return 1
  [[ "$file" != tests/* ]] || return 1
  [[ "$file" != */tests/* ]] || return 1
  return 0
}

run_self_test() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  cat >"$tmp/sample.rs" <<'RUST'
// ignored
/// ignored
//! ignored

fn one() {}
/*
fn ignored_block() {}
*/
fn two() {
    let _x = 1; // counted
}
RUST

  local count
  count="$(count_file "$tmp/sample.rs")"
  if [[ "$count" != "4" ]]; then
    echo "self-test failed: expected 4 counted lines, got $count" >&2
    return 1
  fi

  if is_prod_rust_file "src/foo_tests.rs"; then
    echo "self-test failed: *_tests.rs should be excluded" >&2
    return 1
  fi
  if is_prod_rust_file "tests/integration.rs"; then
    echo "self-test failed: tests/ files should be excluded" >&2
    return 1
  fi
  if ! is_prod_rust_file "src/foo.rs"; then
    echo "self-test failed: src/foo.rs should be included" >&2
    return 1
  fi
  echo "self-test ok"
}

if [[ "$self_test" -eq 1 ]]; then
  run_self_test
  exit
fi

tracked_files() {
  if [[ $# -eq 0 ]]; then
    git ls-files --cached --others --exclude-standard '*.rs'
    return
  fi

  local path
  for path in "$@"; do
    if [[ -f "$path" ]]; then
      git ls-files --cached --others --exclude-standard --error-unmatch "$path" 2>/dev/null || true
    elif [[ -d "$path" ]]; then
      git ls-files --cached --others --exclude-standard "$path/**/*.rs" "$path/*.rs" 2>/dev/null || true
    else
      echo "path not found: $path" >&2
      return 2
    fi
  done | sort -u
}

soft_hits=()
hard_hits=()
while IFS= read -r file; do
  is_prod_rust_file "$file" || continue
  # A tracked file may be deleted in the current worktree before the deletion
  # is staged. It is no longer a production module and must not be counted.
  [[ -f "$file" ]] || continue
  count="$(count_file "$file")"
  if (( count > hard )); then
    hard_hits+=("$(printf '%s\t%s' "$count" "$file")")
  elif (( count > soft )); then
    soft_hits+=("$(printf '%s\t%s' "$count" "$file")")
  fi
done < <(tracked_files "$@")

if (( ${#soft_hits[@]} > 0 )); then
  echo "" >&2
  echo "NOTE: Rust module(s) above the ${soft}-line soft budget (advisory, not blocking):" >&2
  printf '  %s\n' "${soft_hits[@]}" >&2
  echo "Consider whether the module is still cohesive; split into focused siblings if not." >&2
fi

if (( ${#hard_hits[@]} > 0 )); then
  echo "" >&2
  echo "NO MONOLITHS: Rust module(s) above the ${hard}-line HARD limit (must split):" >&2
  printf '  %s\n' "${hard_hits[@]}" >&2
  echo "Split into small focused modules (sibling foo.rs files, no mod.rs)." >&2
  exit 1
fi

exit 0
