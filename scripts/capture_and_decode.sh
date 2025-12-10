#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: capture_and_decode.sh <ssh_user@host> <tcpdump_host> <port>

Example:
  ./scripts/capture_and_decode.sh user@integration.example.com 192.168.1.10 1234

Notes:
  - <port> is used in both the tcpdump filter and the pcap2fix --port argument.
  - Assumes fixdecoder and pcap2fix binaries are available at ./target/release/.
USAGE
}

if [[ $# -lt 3 ]]; then
  usage
  exit 1
fi

SSH_TARGET="$1"
TCP_HOST="$2"
PORT="$3"
shift 3
FIXDECODER_ARGS=("$@")

REMOTE_CMD="sudo tcpdump -U -n -s0 -i any -w - \"(host ${TCP_HOST} and port ${PORT}) and tcp[((tcp[12] & 0xf0) >> 2):4] = 0x383d4649 and tcp[((tcp[12] & 0xf0) >> 2) + 4] = 0x58\""

PCAP2FIX_BIN="${PCAP2FIX_BIN:-./target/release/pcap2fix}"
FIXDECODER_BIN="${FIXDECODER_BIN:-./target/release/fixdecoder}"

if [[ ! -x "${PCAP2FIX_BIN}" || ! -x "${FIXDECODER_BIN}" ]]; then
  echo "error: expected binaries at ${PCAP2FIX_BIN} and ${FIXDECODER_BIN}. Build them first (cargo build --release)." >&2
  exit 1
fi

ssh "${SSH_TARGET}" "${REMOTE_CMD}" \
  | "${PCAP2FIX_BIN}" --port "${PORT}" \
  | "${FIXDECODER_BIN}" --follow "${FIXDECODER_ARGS[@]}"
