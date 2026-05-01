#!/usr/bin/env bash
set -euo pipefail

# Requer: bindgen, clang, cmake
# Uso: ./scripts/regenerate-bindings.sh
#
# Este script regenera os bindings FFI prebuilt em
# cyclonedds-rust-sys/src/prebuilt_bindings.rs a partir dos headers C do
# CycloneDDS.  Deve ser executado por mantenedores quando a C API mudar.

cd "$(dirname "$0")/.."

# Garante que o source C existe
if [[ ! -f "cyclonedds-src/src/cyclonedds/CMakeLists.txt" ]]; then
    echo "ERROR: CycloneDDS source not found in cyclonedds-src/src/cyclonedds/"
    exit 1
fi

# Diretórios de include (ajuste se a estrutura mudar)
CORE_INCLUDE="$(pwd)/cyclonedds-src/src/cyclonedds/src/core/ddsc/include"
DDSRT_INCLUDE="$(pwd)/cyclonedds-src/src/cyclonedds/src/ddsrt/include"

# O config header gerado pelo cmake normalmente fica em:
#   target/debug/build/cyclonedds-src-*/out/cyclonedds-build/src/core/include
# ou similar.  Se não existir, o bindgen pode falhar em encontrar alguns defines.
# O script tenta localizar automaticamente.
BUILD_INCLUDE=""
for dir in target/*/build/cyclonedds-src-*/out/cyclonedds-build/src/core/include; do
    if [[ -d "$dir" ]]; then
        BUILD_INCLUDE="$(pwd)/$dir"
        break
    fi
done

CLANG_ARGS=(
    "-I${CORE_INCLUDE}"
    "-I${DDSRT_INCLUDE}"
)

if [[ -n "${BUILD_INCLUDE}" ]]; then
    CLANG_ARGS+=("-I${BUILD_INCLUDE}")
fi

echo "Regenerating bindings..."
echo "  core include : ${CORE_INCLUDE}"
echo "  ddsrt include: ${DDSRT_INCLUDE}"
echo "  build include: ${BUILD_INCLUDE:-<not found>}"

bindgen \
    cyclonedds-rust-sys/wrapper.h \
    --output cyclonedds-rust-sys/src/prebuilt_bindings.rs \
    --allowlist-function '^dds_.*' \
    --allowlist-type '^dds_.*' \
    --allowlist-var '^DDS_.*' \
    --blocklist-type '__.*' \
    --clang-arg="${CLANG_ARGS[0]}" \
    --clang-arg="${CLANG_ARGS[1]}" \
    ${BUILD_INCLUDE:+--clang-arg="-I${BUILD_INCLUDE}"} \
    --size_t-is-usize \
    --no-layout-tests \
    --generate-inline-functions

echo "Bindings regenerated at cyclonedds-rust-sys/src/prebuilt_bindings.rs"
