#!/usr/bin/env bash
#
# fetch-assets.sh — download models and build/fetch engine binaries for the
# PatientVault USB app (LLM + STT + TTS).
#
# What it produces (idempotent — existing files are skipped):
#   resources/bin/            llama-server, whisper-stream, whisper-cli, piper (+ their .so libs, espeak-ng-data)
#   $MODELS_DIR/llm/          qwen2.5-7b-instruct-q4_k_m.gguf
#   $MODELS_DIR/whisper/      ggml-base.bin
#   $MODELS_DIR/piper/        en/es/zh voices (.onnx + .onnx.json)
#
# Binaries always go to resources/bin so they are bundled next to the app (on the
# USB drive in the portable model, or under /usr/lib/patient-vault/bin for a .deb).
# Models default to resources/models for development. For a portable USB build,
# run with MODELS_DIR pointing at the app's data dir on the drive, e.g.:
#   MODELS_DIR="/media/$USER/PVAULT/patient-vault-data/models" ./scripts/fetch-assets.sh models
#
# Usage:
#   ./scripts/fetch-assets.sh                 # everything
#   ./scripts/fetch-assets.sh models          # models only
#   ./scripts/fetch-assets.sh binaries        # binaries only
#   ./scripts/fetch-assets.sh llama|whisper|piper   # one engine binary
#
# NOTE: The download URLs and the Piper release tag below are pinned but should
# be re-verified over time (see SETUP.md). Build the llama.cpp / whisper.cpp
# binaries against a compatible CPU baseline for the target machines (see SETUP.md).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_TAURI="$(cd "$SCRIPT_DIR/.." && pwd)/src-tauri"
RES="$SRC_TAURI/resources"
BIN_DIR="$RES/bin"
MODELS_DIR="${MODELS_DIR:-$RES/models}"

# Pinned sources (verify per SETUP.md before a release).
QWEN_URL="https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf"
QWEN_DEST="$MODELS_DIR/llm/qwen2.5-7b-instruct-q4_k_m.gguf"
WHISPER_MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
WHISPER_MODEL_DEST="$MODELS_DIR/whisper/ggml-base.bin"
PIPER_VOICES_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"
PIPER_RELEASE="https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz"

LLAMA_REPO="https://github.com/ggml-org/llama.cpp"
WHISPER_REPO="https://github.com/ggml-org/whisper.cpp"
BUILD_ROOT="${BUILD_ROOT:-$SRC_TAURI/.build-engines}"

log()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!! \033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31mxx \033[0m %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "Missing required tool: $1 (install it and re-run)"; }

# download URL DEST — resumable, skips if the destination already exists.
download() {
  local url="$1" dest="$2"
  if [[ -f "$dest" ]]; then
    log "skip (exists): ${dest#"$RES/"}"
    return 0
  fi
  mkdir -p "$(dirname "$dest")"
  log "download: $(basename "$dest")"
  if command -v curl >/dev/null 2>&1; then
    curl -L --fail --retry 3 -C - -o "$dest.part" "$url"
  else
    need wget
    wget -c -O "$dest.part" "$url"
  fi
  mv "$dest.part" "$dest"
}

fetch_models() {
  log "Models → $MODELS_DIR"
  download "$QWEN_URL" "$QWEN_DEST"
  download "$WHISPER_MODEL_URL" "$WHISPER_MODEL_DEST"

  # Piper voices: name → repo subpath
  local -a voices=(
    "en_US-lessac-medium:en/en_US/lessac/medium"
    "es_MX-ald-medium:es/es_MX/ald/medium"
    "zh_CN-huayan-medium:zh/zh_CN/huayan/medium"
  )
  local entry name path
  for entry in "${voices[@]}"; do
    name="${entry%%:*}"
    path="${entry#*:}"
    download "$PIPER_VOICES_BASE/$path/$name.onnx" "$MODELS_DIR/piper/$name.onnx"
    download "$PIPER_VOICES_BASE/$path/$name.onnx.json" "$MODELS_DIR/piper/$name.onnx.json"
  done
}

# Build a cmake target from a git repo and copy the binary (+ sibling .so libs).
build_cmake_engine() {
  local repo="$1" dir="$2"
  shift 2
  local -a targets=("$@")

  need git; need cmake
  mkdir -p "$BUILD_ROOT"
  local src="$BUILD_ROOT/$dir"
  if [[ ! -d "$src" ]]; then
    log "clone: $repo"
    git clone --depth 1 "$repo" "$src"
  fi

  log "cmake configure: $dir"
  cmake -S "$src" -B "$src/build" -DCMAKE_BUILD_TYPE=Release "${CMAKE_EXTRA[@]:-}"
  log "cmake build: ${targets[*]}"
  cmake --build "$src/build" --config Release -j"$(nproc)" "${targets[@]/#/--target=}"

  mkdir -p "$BIN_DIR"
  local t found
  for t in "${targets[@]}"; do
    found="$(find "$src/build" -type f -name "$t" -perm -u+x | head -n1 || true)"
    [[ -n "$found" ]] || die "Built target not found: $t"
    cp -f "$found" "$BIN_DIR/"
    # Copy shared libraries that live next to the binary (llama/whisper split builds).
    find "$(dirname "$found")" -maxdepth 1 -name '*.so*' -exec cp -f {} "$BIN_DIR/" \; 2>/dev/null || true
  done
}

fetch_llama() {
  log "Engine: llama.cpp (llama-server)"
  if [[ -x "$BIN_DIR/llama-server" ]]; then log "skip (exists): bin/llama-server"; return 0; fi
  CMAKE_EXTRA=(-DLLAMA_CURL=OFF -DGGML_NATIVE=OFF)
  build_cmake_engine "$LLAMA_REPO" "llama.cpp" llama-server
}

fetch_whisper() {
  log "Engine: whisper.cpp (whisper-stream, whisper-cli)"
  if [[ -x "$BIN_DIR/whisper-stream" && -x "$BIN_DIR/whisper-cli" ]]; then
    log "skip (exists): bin/whisper-stream, bin/whisper-cli"; return 0
  fi
  command -v sdl2-config >/dev/null 2>&1 || warn "SDL2 dev headers not found — whisper-stream needs libsdl2-dev (apt install libsdl2-dev)."
  CMAKE_EXTRA=(-DWHISPER_SDL2=ON -DWHISPER_BUILD_EXAMPLES=ON -DGGML_NATIVE=OFF)
  build_cmake_engine "$WHISPER_REPO" "whisper.cpp" whisper-stream whisper-cli
}

fetch_piper() {
  log "Engine: piper (rhasspy v1.2.0 CLI)"
  if [[ -x "$BIN_DIR/piper" ]]; then log "skip (exists): bin/piper"; return 0; fi
  need tar
  mkdir -p "$BIN_DIR" "$BUILD_ROOT"
  local tarball="$BUILD_ROOT/piper.tar.gz"
  download "$PIPER_RELEASE" "$tarball"
  log "extract piper → bin/"
  local tmp; tmp="$(mktemp -d)"
  tar -xzf "$tarball" -C "$tmp"
  # The archive contains a top-level `piper/` directory.
  cp -rf "$tmp/piper/." "$BIN_DIR/"
  rm -rf "$tmp"
  chmod +x "$BIN_DIR/piper" || true
}

main() {
  local what="${1:-all}"
  case "$what" in
    all)       fetch_models; fetch_llama; fetch_whisper; fetch_piper ;;
    models)    fetch_models ;;
    binaries)  fetch_llama; fetch_whisper; fetch_piper ;;
    llama)     fetch_llama ;;
    whisper)   fetch_whisper ;;
    piper)     fetch_piper ;;
    *)         die "Unknown target '$what' (use: all|models|binaries|llama|whisper|piper)" ;;
  esac
  log "Done. Binaries in resources/bin, models in $MODELS_DIR"
}

main "$@"
