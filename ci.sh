#!/usr/bin/env bash
# fixdecoder — unified CI helper for the Rust implementation

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${ROOT_DIR}"

# Load optional project-level environment (e.g., SONAR_TOKEN) if present.
if [[ -f "./PROJECT_ENVIRONMENT_VARIABLES.env" ]]; then
  # shellcheck disable=SC1091
  source "./PROJECT_ENVIRONMENT_VARIABLES.env"
  if [[ -n "${SONAR_TOKEN:-}" ]]; then
    printf "\nLoaded SONAR_TOKEN from PROJECT_ENVIRONMENT_VARIABLES.env\n"
  else
    printf "\nWarning: SONAR_TOKEN not set after sourcing PROJECT_ENVIRONMENT_VARIABLES.env\n" >&2
  fi
fi

function log() {
  printf "\n\033[1;32m%s\033[0m\n" "$1"
}

function warn() {
  printf "\n\033[38;5;214m%s\033[0m\n" "$1"
}

setup_done=false
function cmd_setup_environment() {
  if [[ "${setup_done}" == true ]]; then
    return
  fi
  log ">> Ensuring Rust toolchain and coverage tools"
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is not on PATH. Please install Rust (https://www.rust-lang.org/tools/install)." >&2
    exit 1
  fi
  if ! rustup component list --installed | grep -q 'llvm-tools-preview'; then
    log ">> Installing llvm-tools-preview component"
    rustup component add llvm-tools-preview >/dev/null
  fi
  if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    log ">> Installing cargo-llvm-cov"
    cargo install cargo-llvm-cov --locked --quiet
  fi
  setup_done=true
}

function ensure_sonar_scanner() {
  if command -v sonar-scanner >/dev/null 2>&1; then
    return
  fi
  log ">> Installing sonar-scanner CLI locally"
  local version="5.0.1.3006"
  local tools_dir="${ROOT_DIR}/target/tools"
  local archive
  local urls=()
  local os="$(uname -s | tr '[:upper:]' '[:lower:]')"

  mkdir -p "${tools_dir}"

  case "${os}" in
    linux*)
      archive="/tmp/sonar-scanner-${version}-linux-x64.zip"
      urls+=(
        "https://binaries.sonarsource.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-${version}-linux-x64.zip"
        "https://sonarsource.bintray.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-${version}-linux-x64.zip"
      )
      ;;
    darwin*)
      archive="/tmp/sonar-scanner-${version}-macosx.zip"
      urls+=(
        "https://binaries.sonarsource.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-${version}-macosx.zip"
        "https://sonarsource.bintray.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-${version}-macosx.zip"
      )
      ;;
    msys*|mingw*|cygwin*)
      archive="/tmp/sonar-scanner-${version}-windows.zip"
      urls+=(
        "https://binaries.sonarsource.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-${version}-windows.zip"
        "https://sonarsource.bintray.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-${version}-windows.zip"
      )
      ;;
    *)
      warn "Unsupported OS for auto-installing sonar-scanner (${os}); please install manually."
      return 1
      ;;
  esac

  local downloaded=""
  for url in "${urls[@]}"; do
    log "   attempting download: ${url}"
    if curl -fsSL -o "${archive}" "${url}"; then
      downloaded="${archive}"
      break
    fi
  done

  if [[ -z "${downloaded}" ]]; then
    warn "Failed to download sonar-scanner; install manually or ensure it is on PATH."
    return 1
  fi

  unzip -qo "${downloaded}" -d "${tools_dir}"

  local candidate
  candidate="$(find "${tools_dir}" -maxdepth 3 -type f \( -name "sonar-scanner" -o -name "sonar-scanner.bat" \) | head -n 1 || true)"
  if [[ -z "${candidate}" ]]; then
    warn "sonar-scanner executable not found after extraction in ${tools_dir}"
    return 1
  fi

  local bin_dir
  bin_dir="$(dirname "${candidate}")"
  export PATH="${bin_dir}:${PATH}"
  log "   sonar-scanner installed locally at ${candidate}"
}

metadata_ready=false
function crate_version() {
  grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/'
}

function download_fix_specs() {
  log ">> Ensuring FIX XML specs are present"
  local resources_dir="${ROOT_DIR}/resources"
  mkdir -p "${resources_dir}"

  # Align with embedded dictionaries: 40,41,42,43,44,50,50SP1,50SP2,T11
  local specs=(
    "FIX40.xml"
    "FIX41.xml"
    "FIX42.xml"
    "FIX43.xml"
    "FIX44.xml"
    "FIX50.xml"
    "FIX50SP1.xml"
    "FIX50SP2.xml"
    "FIXT11.xml"
  )

  for spec in "${specs[@]}"; do
    local dest="${resources_dir}/${spec}"
    local url="https://raw.githubusercontent.com/quickfix/quickfix/master/spec/${spec}"
    if [[ -f "${dest}" ]]; then
      continue
    fi
    log "   downloading ${spec}"
    if ! curl -fsSL -o "${dest}" "${url}"; then
      echo "Failed to download ${spec} from ${url}" >&2
      exit 1
    fi
  done
}

function ensure_build_metadata() {
  if [[ "${metadata_ready}" == true ]]; then
    return
  fi

  local branch commit url
  branch=${FIXDECODER_BRANCH:-}
  commit=${FIXDECODER_COMMIT:-}
  url=${FIXDECODER_GIT_URL:-}

  if [[ -z "${branch}" ]]; then
    branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "main")
  fi
  if [[ -z "${commit}" ]]; then
    commit=$(git rev-parse --short HEAD 2>/dev/null || echo "0000000")
  fi
  if [[ -z "${url}" ]]; then
    url=$(git remote get-url origin 2>/dev/null || echo "https://github.com/stephenlclarke/fixdecoder.git")
  fi
  local version
  if [[ -n "${FIXDECODER_VERSION:-}" ]]; then
    version="${FIXDECODER_VERSION}"
  else
    version=$(git tag --list 'v[0-9]*' --sort=-version:refname | head -n 1 || true)
    if [[ -z "${version}" ]]; then
      local crate_ver
      if ! crate_ver=$(crate_version); then
        echo "Unable to determine crate version from Cargo.toml" >&2
        exit 1
      fi
      version="v${crate_ver}"
    fi
  fi

  export FIXDECODER_BRANCH="${branch}"
  export FIXDECODER_COMMIT="${commit}"
  export FIXDECODER_GIT_URL="${url}"
  export FIXDECODER_VERSION="${version}"

  metadata_ready=true
}

function generate_sensitive_tags() {
  log ">> Regenerating sensitive tag map"
  download_fix_specs
  cargo run --quiet --bin generate_sensitive_tags >/dev/null
}

function cmd_build() {
  cmd_setup_environment
  ensure_build_metadata
  generate_sensitive_tags
  log ">> Building (debug)"
  cargo fmt --all
  cargo build
  log ">> Debug build complete"
}

function cmd_build_release() {
  cmd_setup_environment
  ensure_build_metadata
  generate_sensitive_tags
  log ">> Building (release)"
  cargo fmt --all
  cargo build --release
  log ">> Release build complete"
}

function cmd_scan() {
  cmd_setup_environment
  ensure_build_metadata
  log ">> Running cargo fmt --check"
  cargo fmt --all --check
  log ">> Running cargo clippy"
  cargo clippy --all-targets -- -D warnings
  log ">> Running cargo audit (if available)"
  if command -v cargo-audit >/dev/null 2>&1; then
    cargo audit
  else
    warn "cargo-audit not installed; skipping security scan"
  fi
}

function cmd_clean() {
  cmd_setup_environment
  log ">> Cleaning workspace"
  cargo clean
}

function cmd_coverage() {
  cmd_setup_environment
  ensure_build_metadata
  generate_sensitive_tags
  log ">> Running coverage (cargo llvm-cov --cobertura)"
  if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    warn "cargo-llvm-cov not installed; install with: cargo install cargo-llvm-cov"
    return 1
  fi
  mkdir -p target/coverage
  cargo llvm-cov clean --workspace >/dev/null 2>&1 || true
  cargo llvm-cov --workspace --cobertura \
    --ignore-filename-regex 'src/fix/sensitive.rs|src/bin/generate_sensitive_tags.rs' \
    --output-path target/coverage/coverage.xml
  log ">> Coverage report written to target/coverage/coverage.xml"
}

function cmd_sonar() {
  cmd_setup_environment
  ensure_build_metadata
  if [[ ! -f target/coverage/coverage.xml ]]; then
    warn "Coverage report not found at target/coverage/coverage.xml; running coverage first"
    cmd_coverage
  fi
  ensure_sonar_scanner
  log ">> Running sonar-scanner"
  sonar-scanner
}

function usage() {
  cat <<EOF
Usage: ./ci.sh <command> [command...]
Commands (can be chained; dependencies auto-run once):
  setup-environment   Ensure the Rust toolchain is available
  build               Run generator and cargo build (debug)
  build-release       Run generator and cargo build --release
  scan                Run cargo fmt --check, cargo clippy, cargo audit
  coverage            Run tests with coverage (cargo llvm-cov --cobertura)
  sonar               Run sonar-scanner (requires coverage.xml)
  clean               Run cargo clean
EOF
}

case "${1:-}" in
  "") usage ;;
  *) ;;
esac

requested=( "$@" )

ran_steps=""
function already_ran() {
  [[ " ${ran_steps} " == *" $1 "* ]]
}

function mark_ran() {
  ran_steps="${ran_steps} $1"
}

function run_step() {
  local step="$1"
  if already_ran "${step}"; then
    return
  fi
  case "${step}" in
    setup-environment) cmd_setup_environment ;;
    build) cmd_build ;;
    build-release) cmd_build_release ;;
    scan) cmd_scan ;;
    coverage) cmd_coverage ;;
    sonar) cmd_sonar ;;
    clean) cmd_clean ;;
    *)
      echo "Unknown command: ${step}" >&2
      usage
      exit 1
      ;;
  esac
  mark_ran "${step}"
}

function require_deps() {
  local step="$1"
  case "${step}" in
    coverage)
      run_step build
      ;;
    sonar)
      run_step build
      run_step coverage
      ;;
    *)
      ;;
  esac
}

# Run clean first if requested, to avoid cleaning after other steps.
declare -a clean_first=()
declare -a rest=()
for step in "${requested[@]}"; do
  if [[ "${step}" == "clean" ]]; then
    clean_first+=(clean)
  else
    rest+=("${step}")
  fi
done

for step in "${clean_first[@]-}"; do
  require_deps "${step}"
  run_step "${step}"
done

for step in "${rest[@]-}"; do
  require_deps "${step}"
  run_step "${step}"
done
