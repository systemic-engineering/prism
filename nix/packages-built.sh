# checks.packages-built: the binaries, as packages, run.
#
#   bash packages-built.sh <echo package> <prism-mcp package>
#
# An empty argument is a package the flake doesn't build. Each package's
# bin/ holds exactly its one binary, so the store path names that
# executable and nothing else (Seam's beam-vsm-audit: "a port executable
# by hash", prism-028 Response §3 (b)).
#
# - echo's `prism`: any argument shape but `echo <3 or 4 paths>` is usage
#   on stderr, nothing on stdout, exit 2 (docs/spec/echo.md §10.1).
# - `prism-mcp`: one `initialize` line on stdin is answered on stdout with
#   the result (protocolVersion 2025-06-18, serverInfo prism-mcp), then
#   end of input exits 0 (docs/spec/mcp.md §2, §4).
set -euo pipefail

echo_pkg=$1
mcp_pkg=$2
fail() {
  echo "✖ packages-built: $*" >&2
  exit 1
}

[ -n "$echo_pkg" ] || fail "no package echo"
[ -n "$mcp_pkg" ] || fail "no package prism-mcp"

only() {
  local pkg=$1 name=$2 got
  got=$(ls "$pkg/bin" 2>/dev/null | tr '\n' ' ')
  [ "$got" = "$name " ] || fail "$pkg/bin holds '$got', not '$name'"
}
only "$echo_pkg" prism
only "$mcp_pkg" prism-mcp

usage="usage: prism echo <source> <certificate> <record> [<score>]"
for args in "" "--help" "echo" "echo a b" "echo a b c d e" "nope a b c"; do
  set +e
  # shellcheck disable=SC2086 # the words are the argument shape
  out=$("$echo_pkg/bin/prism" $args 2>stderr)
  code=$?
  set -e
  [ "$code" = 2 ] || fail "prism $args: exit $code, not 2"
  [ -z "$out" ] || fail "prism $args: wrote to stdout: $out"
  [ "$(cat stderr)" = "$usage" ] || fail "prism $args: stderr was: $(cat stderr)"
done

init='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"packages-built","version":"0"}}}'
set +e
out=$(printf '%s\n' "$init" | "$mcp_pkg/bin/prism-mcp" 2>stderr)
code=$?
set -e
[ "$code" = 0 ] || fail "prism-mcp: exit $code, not 0"
[ ! -s stderr ] || fail "prism-mcp: wrote to stderr: $(cat stderr)"
[ "$(printf '%s\n' "$out" | wc -l | tr -d ' ')" = 1 ] || fail "prism-mcp: not one line: $out"
for want in '"jsonrpc":"2.0"' '"id":1' '"result":' '"protocolVersion":"2025-06-18"' '"serverInfo":{"name":"prism-mcp"'; do
  case "$out" in
    *"$want"*) ;;
    *) fail "prism-mcp: no $want in: $out" ;;
  esac
done

echo "✔ packages-built: echo's prism (usage, exit 2) and prism-mcp (initialize) run"
