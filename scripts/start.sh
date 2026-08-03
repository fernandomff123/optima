#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cleanup() {
  if [[ -n "${api_pid:-}" ]]; then
    kill "$api_pid" 2>/dev/null || true
    wait "$api_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

if curl --fail --silent --output /dev/null http://127.0.0.1:3100/api/health; then
  echo "A porta 3100 já está ocupada por outra API. Termina o processo antigo antes de continuar." >&2
  exit 1
fi

echo "A iniciar API em http://127.0.0.1:3100"
RUSTFLAGS='' cargo run --bin web_server &
api_pid=$!

echo "A aguardar que a API fique disponível"
for attempt in {1..240}; do
  if curl --fail --silent --output /dev/null http://127.0.0.1:3100/api/health; then
    break
  fi
  if ! kill -0 "$api_pid" 2>/dev/null; then
    wait "$api_pid"
    exit $?
  fi
  if [[ "$attempt" -eq 240 ]]; then
    echo "A API não ficou disponível dentro de 60 segundos" >&2
    exit 1
  fi
  sleep 0.25
done

cd web
echo "A iniciar interface em http://127.0.0.1:8180"
env -u NO_COLOR trunk serve --open=false
