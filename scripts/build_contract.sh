#!/bin/bash
# Amitis Network — CosmWasm Contract Build Script
# No Docker required. Run this from your contract directory.
#
# Requirements:
#   - Rust with wasm32-unknown-unknown target
#   - wasm-opt v121+ at /usr/local/bin/wasm-opt
#
# Install wasm-opt v121:
#   wget https://github.com/WebAssembly/binaryen/releases/download/version_121/binaryen-version_121-x86_64-linux.tar.gz
#   tar xzf binaryen-version_121-x86_64-linux.tar.gz
#   sudo cp binaryen-version_121/bin/* /usr/local/bin/
#
# Install Rust target:
#   rustup target add wasm32-unknown-unknown

set -e

CONTRACT_NAME=$(basename $(pwd) | tr '-' '_')
OUTPUT="artifacts/${CONTRACT_NAME}.wasm"

mkdir -p artifacts

echo "=== Building $CONTRACT_NAME ==="
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --lib

echo "=== Optimizing and lowering bulk memory ==="
/usr/local/bin/wasm-opt -Os \
  --enable-bulk-memory \
  --llvm-memory-copy-fill-lowering \
  --enable-sign-ext \
  target/wasm32-unknown-unknown/release/${CONTRACT_NAME}.wasm \
  -o $OUTPUT

echo "=== Verifying no bulk memory instructions ==="
COUNT=$(/usr/local/bin/wasm-opt --print $OUTPUT 2>&1 | grep -c "memory.copy\|memory.fill" || true)
if [ "$COUNT" -gt "0" ]; then
  echo "ERROR: $COUNT bulk memory instructions remain"
  exit 1
fi

echo "=== Done ==="
ls -lh $OUTPUT
echo "Checksum: $(sha256sum $OUTPUT | cut -d' ' -f1)"
echo ""
echo "Upload to Amitis:"
echo "  amisd tx wasm store $OUTPUT --from ops --chain-id amitis-network --node https://rpc.amitis.network:443 --keyring-backend test --home ~/.amitis --gas 3000000 --fees 150000uamts -y"
