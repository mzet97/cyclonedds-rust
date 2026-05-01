#!/usr/bin/env bash
# Generate self-signed certificates for DDS Security testing.
#
# Usage:
#   cd examples/security
#   ../../scripts/generate-certs.sh
#
# Requires: openssl

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTDIR="${DIR}/../examples/security"
mkdir -p "${OUTDIR}"

cd "${OUTDIR}"

echo "Generating DDS Security certificates in $(pwd)..."

# ---- Identity CA ----
openssl req -x509 -newkey rsa:2048 \
    -keyout identity_ca_key.pem -out identity_ca_cert.pem \
    -days 365 -nodes \
    -subj "/C=BR/O=CycloneDDS/CN=Identity CA" \
    2>/dev/null

# ---- Permissions CA ----
openssl req -x509 -newkey rsa:2048 \
    -keyout permissions_ca_key.pem -out permissions_ca_cert.pem \
    -days 365 -nodes \
    -subj "/C=BR/O=CycloneDDS/CN=Permissions CA" \
    2>/dev/null

# ---- Participant identity ----
openssl req -newkey rsa:2048 \
    -keyout participant_key.pem -out participant_req.pem \
    -days 365 -nodes \
    -subj "/C=BR/O=CycloneDDS/CN=Participant" \
    2>/dev/null

openssl x509 -req -in participant_req.pem \
    -CA identity_ca_cert.pem -CAkey identity_ca_key.pem \
    -CAcreateserial -out participant_cert.pem -days 365 \
    2>/dev/null

# ---- Clean up ----
rm -f participant_req.pem identity_ca_key.pem permissions_ca_key.pem *.srl

echo "Done. Files in ${OUTDIR}:"
ls -la *.pem 2>/dev/null || true
