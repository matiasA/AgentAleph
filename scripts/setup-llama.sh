#!/usr/bin/env bash
# Descarga el binario precompilado de llama.cpp (llama-server + librerías) y lo
# coloca en src-tauri/binaries/llama-linux-x64/, que es donde la app lo busca.
#
# Estos binarios NO se versionan en git (son pesados). Corré este script una vez
# tras clonar, y la CI lo corre antes de compilar los instalables.
#
# Variables de entorno:
#   LLAMA_TAG     tag de release de llama.cpp     (default: b9754)
#   LLAMA_FLAVOR  variante del build              (default: vulkan-x64)
#                 opciones: x64 (CPU), vulkan-x64, rocm-7.2-x64, sycl-fp16-x64, …
#   FORCE=1       re-descarga aunque ya exista
#
# Uso:
#   ./scripts/setup-llama.sh
#   LLAMA_FLAVOR=x64 ./scripts/setup-llama.sh    # build CPU puro
set -euo pipefail

LLAMA_TAG="${LLAMA_TAG:-b9754}"
LLAMA_FLAVOR="${LLAMA_FLAVOR:-vulkan-x64}"
FORCE="${FORCE:-0}"

# Raíz del repo (este script vive en scripts/).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/src-tauri/binaries/llama-linux-x64"

ASSET="llama-${LLAMA_TAG}-bin-ubuntu-${LLAMA_FLAVOR}.tar.gz"
URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_TAG}/${ASSET}"

if [[ -x "$DEST/llama-server" && "$FORCE" != "1" ]]; then
  echo "✓ llama-server ya está en $DEST (usá FORCE=1 para re-descargar)"
  exit 0
fi

echo "→ Descargando $ASSET"
echo "  $URL"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! curl -fSL --retry 3 -o "$TMP/llama.tar.gz" "$URL"; then
  echo "✗ No se pudo descargar $URL" >&2
  echo "  Verificá el tag/variante en https://github.com/ggml-org/llama.cpp/releases" >&2
  exit 1
fi

echo "→ Extrayendo"
tar -xzf "$TMP/llama.tar.gz" -C "$TMP"

# El tarball extrae a un único directorio (p.ej. llama-b9754/). Aplanamos su
# contenido dentro de DEST.
SRCDIR="$(find "$TMP" -mindepth 1 -maxdepth 1 -type d | head -1)"
if [[ -z "$SRCDIR" || ! -x "$SRCDIR/llama-server" ]]; then
  echo "✗ El tarball no contiene llama-server donde se esperaba" >&2
  exit 1
fi

mkdir -p "$DEST"
cp -af "$SRCDIR"/. "$DEST"/
chmod +x "$DEST/llama-server" 2>/dev/null || true

echo "✓ Listo: $(du -sh "$DEST" | cut -f1) en $DEST"
"$DEST/llama-server" --version 2>&1 | head -1 || true
