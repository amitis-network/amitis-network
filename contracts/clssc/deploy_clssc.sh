#!/bin/bash
# CLSSC Deploy Script — Amitis Network
# Run from build VM (192.168.12.201) or any node with amisd + ops key
#
# Usage:
#   ./deploy_clssc.sh [--testnet]
#
# Prerequisites:
#   - amisd binary in PATH or at ~/amitis-network/build/amisd
#   - ops key imported to keyring: amisd keys show ops --home ~/.amitis
#   - wasm artifact at contracts/clssc/artifacts/clssc.wasm

set -e

BINARY="${AMISD_BIN:-$HOME/amitis-network/build/amisd}"
HOME_DIR="$HOME/.amitis"
CHAIN_ID="amitis-network"
NODE="https://rpc.amitis.network:443"
FROM="ops"
FEES="150000uamts"
KEYRING="--keyring-backend test"
WASM="contracts/clssc/artifacts/clssc.wasm"

# Admin = ops wallet address
ADMIN=$($BINARY keys show ops --home $HOME_DIR $KEYRING -a)
echo "Admin address: $ADMIN"

# ── Step 1: Build ─────────────────────────────────────────────────────────────
echo ""
echo "=== Step 1: Build CLSSC contract ==="
cd contracts/clssc
bash ../../scripts/build_contract.sh
cd ../..

# ── Step 2: Upload ────────────────────────────────────────────────────────────
echo ""
echo "=== Step 2: Upload CLSSC wasm to chain ==="
UPLOAD_TX=$($BINARY tx wasm store $WASM \
  --from $FROM \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --home $HOME_DIR \
  $KEYRING \
  --gas 3000000 \
  --fees $FEES \
  -y \
  --output json 2>&1)

echo "Upload tx: $UPLOAD_TX"
UPLOAD_TXHASH=$(echo $UPLOAD_TX | python3 -c "import sys,json; print(json.load(sys.stdin)['txhash'])" 2>/dev/null || echo "")
echo "TX Hash: $UPLOAD_TXHASH"

echo "Waiting 8s for tx to be included..."
sleep 8

# Get code ID
CODE_ID=$($BINARY query tx $UPLOAD_TXHASH \
  --node $NODE \
  --chain-id $CHAIN_ID \
  --home $HOME_DIR \
  --output json 2>/dev/null | \
  python3 -c "
import sys, json
tx = json.load(sys.stdin)
for log in tx.get('logs', []):
    for ev in log.get('events', []):
        for attr in ev.get('attributes', []):
            if attr.get('key') == 'code_id':
                print(attr['value'])
                sys.exit(0)
# Try raw_log fallback
raw = tx.get('raw_log', '')
import re
m = re.search(r'code_id.*?(\d+)', raw)
if m: print(m.group(1))
" 2>/dev/null || echo "")

if [ -z "$CODE_ID" ]; then
  echo "Could not auto-detect code ID. Check tx on explorer and set CODE_ID manually."
  echo "  https://amitis.network/explorer.html"
  echo ""
  read -p "Enter Code ID: " CODE_ID
fi

echo "Code ID: $CODE_ID"

# ── Step 3: Instantiate ───────────────────────────────────────────────────────
echo ""
echo "=== Step 3: Instantiate CLSSC ==="

INIT_MSG=$(cat <<EOF
{
  "name": "Meridian Closed-Loop Stablecoin",
  "symbol": "CLSSC",
  "decimals": 6,
  "admin": "$ADMIN",
  "initial_minters": [
    ["$ADMIN", "meridian-ops-initial"]
  ],
  "reserve_uri": "https://api.meridian.finance/clssc/reserve",
  "whitelist_enabled": false,
  "initial_whitelist": [],
  "initial_balances": [],
  "mint_caps": []
}
EOF
)

echo "Init msg: $INIT_MSG"

INST_TX=$($BINARY tx wasm instantiate $CODE_ID "$INIT_MSG" \
  --label "CLSSC-v1" \
  --admin $ADMIN \
  --from $FROM \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --home $HOME_DIR \
  $KEYRING \
  --gas 500000 \
  --fees $FEES \
  -y \
  --output json 2>&1)

echo "Instantiate tx: $INST_TX"
INST_TXHASH=$(echo $INST_TX | python3 -c "import sys,json; print(json.load(sys.stdin)['txhash'])" 2>/dev/null || echo "")
echo "TX Hash: $INST_TXHASH"

echo "Waiting 8s..."
sleep 8

# Get contract address
CONTRACT_ADDR=$($BINARY query wasm list-contract-by-code $CODE_ID \
  --node $NODE \
  --chain-id $CHAIN_ID \
  --home $HOME_DIR \
  --output json 2>/dev/null | \
  python3 -c "import sys,json; contracts=json.load(sys.stdin).get('contracts',[]); print(contracts[-1] if contracts else '')" \
  2>/dev/null || echo "")

if [ -z "$CONTRACT_ADDR" ]; then
  echo "Could not auto-detect contract address."
  read -p "Enter contract address: " CONTRACT_ADDR
fi

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  CLSSC DEPLOYED SUCCESSFULLY"
echo "═══════════════════════════════════════════════════════════"
echo "  Code ID:          $CODE_ID"
echo "  Contract address: $CONTRACT_ADDR"
echo "  Admin:            $ADMIN"
echo "  Symbol:           CLSSC"
echo "  Decimals:         6"
echo "  Whitelist:        disabled (open transfers)"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Save these — add to HANDOFF doc:"
echo "  CLSSC_CODE_ID=$CODE_ID"
echo "  CLSSC_CONTRACT=$CONTRACT_ADDR"
echo ""

# ── Step 4: Verify ────────────────────────────────────────────────────────────
echo "=== Step 4: Verify deployment ==="

echo "Token info:"
$BINARY query wasm contract-state smart $CONTRACT_ADDR \
  '{"token_info":{}}' \
  --node $NODE \
  --chain-id $CHAIN_ID \
  --home $HOME_DIR \
  --output json 2>/dev/null | python3 -m json.tool

echo ""
echo "Config:"
$BINARY query wasm contract-state smart $CONTRACT_ADDR \
  '{"config":{}}' \
  --node $NODE \
  --chain-id $CHAIN_ID \
  --home $HOME_DIR \
  --output json 2>/dev/null | python3 -m json.tool

echo ""
echo "Minters:"
$BINARY query wasm contract-state smart $CONTRACT_ADDR \
  '{"list_minters":{}}' \
  --node $NODE \
  --chain-id $CHAIN_ID \
  --home $HOME_DIR \
  --output json 2>/dev/null | python3 -m json.tool

echo ""
echo "=== Deployment complete ==="
