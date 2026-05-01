#!/usr/bin/env bash
set -euo pipefail

# Script de publicação sequencial dos crates cyclonedds-rust
# Ordem obrigatória: dependências primeiro, crates high-level depois
#
# Requer: cargo login <token> (veja https://crates.io/settings/tokens)
#
# Uso:
#   ./scripts/publish.sh --dry-run   # valida sem publicar
#   ./scripts/publish.sh             # publica de verdade

DRY_RUN=""
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN="--dry-run"
    echo "=== MODO DRY-RUN (não publica) ==="
fi

CRATES=(
    cyclonedds-src
    cyclonedds-rust-sys
    cyclonedds-derive
    cyclonedds-build
    cyclonedds-idlc
    cyclonedds
    cyclonedds-cli
)

for crate in "${CRATES[@]}"; do
    echo ""
    echo "========================================="
    echo "Publicando: $crate"
    echo "========================================="
    cargo publish -p "$crate" $DRY_RUN
    
    if [[ -z "$DRY_RUN" ]]; then
        # Aguarda o crate ficar disponível no registry para os próximos
        echo "Aguardando $crate aparecer no crates.io..."
        sleep 15
    fi
done

echo ""
echo "========================================="
if [[ -n "$DRY_RUN" ]]; then
    echo "DRY-RUN concluído com sucesso"
else
    echo "Publicação concluída!"
fi
echo "========================================="
