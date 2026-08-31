#!/bin/bash
# Meridian PoR Full Deploy Script
# Deploys: por_aggregator + por_onchain (if needed) + wires CLSSC
#
# Usage:
#   ./deploy_por.sh --clssc <CONTRACT_ADDR> [--onchain-reserves]
#
# Prerequisites:
#   - amisd binary + ops key
#   - Rust + wasm-opt at /usr/local/bin/wasm-opt
#   - CLSSC contract already deployed

set -e

BINARY="${AMISD_BIN:-$HOME/amitis-network/build/amisd}"
HOME_DIR="$HOME/.amitis"
CHAIN_ID="amitis-network"
NODE="https://rpc.amitis.network:443"
FROM="ops"
FEES="150000uamts"
KR="--keyring-backend test"
ADMIN=$($BINARY keys show ops --home $HOME_DIR $KR -a)
ORACLE_ADDR=$ADMIN  # Initially ops wallet is the oracle — add proper oracle wallet later

# Parse args
CLSSC_ADDR=""
DEPLOY_ONCHAIN=false
while [[ $# -gt 0 ]]; do
  case $1 in
    --clssc) CLSSC_ADDR="$2"; shift 2;;
    --onchain-reserves) DEPLOY_ONCHAIN=true; shift;;
    *) echo "Unknown arg: $1"; exit 1;;
  esac
done

if [ -z "$CLSSC_ADDR" ]; then
  echo "ERROR: --clssc <CONTRACT_ADDR> is required"
  exit 1
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  Meridian PoR Deploy"
echo "  Admin / Oracle: $ADMIN"
echo "  CLSSC contract: $CLSSC_ADDR"
echo "═══════════════════════════════════════════════════════════════"

# ── Build PoR Aggregator ───────────────────────────────────────────────────────
echo ""
echo "=== Building por_aggregator ==="
cd contracts/por_aggregator
bash ../../scripts/build_contract.sh
AGGREGATOR_WASM="artifacts/por_aggregator.wasm"

# ── Upload aggregator ─────────────────────────────────────────────────────────
echo "=== Uploading por_aggregator ==="
AGG_UPLOAD=$($BINARY tx wasm store $AGGREGATOR_WASM \
  --from $FROM --chain-id $CHAIN_ID --node $NODE \
  --home $HOME_DIR $KR --gas 3000000 --fees $FEES -y --output json 2>&1)
AGG_TXHASH=$(echo $AGG_UPLOAD | python3 -c "import sys,json; print(json.load(sys.stdin)['txhash'])" 2>/dev/null)
echo "Upload TX: $AGG_TXHASH"
sleep 8

AGG_CODE_ID=$($BINARY query tx $AGG_TXHASH --node $NODE --chain-id $CHAIN_ID --home $HOME_DIR --output json 2>/dev/null | \
  python3 -c "
import sys,json,re
tx=json.load(sys.stdin)
for log in tx.get('logs',[]):
  for ev in log.get('events',[]):
    for attr in ev.get('attributes',[]):
      if attr.get('key')=='code_id': print(attr['value']); sys.exit(0)
" 2>/dev/null)

if [ -z "$AGG_CODE_ID" ]; then
  read -p "Enter por_aggregator Code ID: " AGG_CODE_ID
fi
echo "Aggregator Code ID: $AGG_CODE_ID"

# ── Instantiate aggregator ────────────────────────────────────────────────────
echo "=== Instantiating por_aggregator ==="
AGG_INIT=$(cat <<EOF
{
  "admin": "$ADMIN",
  "staleness_threshold_secs": 86400,
  "min_submissions": 1,
  "description": "CLSSC USD Reserve PoR Aggregator",
  "denomination": "USD",
  "initial_oracles": [["$ORACLE_ADDR", "meridian-ops-oracle"]]
}
EOF
)

AGG_INST=$($BINARY tx wasm instantiate $AGG_CODE_ID "$AGG_INIT" \
  --label "meridian-por-aggregator-v1" --admin $ADMIN \
  --from $FROM --chain-id $CHAIN_ID --node $NODE \
  --home $HOME_DIR $KR --gas 500000 --fees $FEES -y --output json 2>&1)
AGG_INST_HASH=$(echo $AGG_INST | python3 -c "import sys,json; print(json.load(sys.stdin)['txhash'])" 2>/dev/null)
echo "Instantiate TX: $AGG_INST_HASH"
sleep 8

AGG_ADDR=$($BINARY query wasm list-contract-by-code $AGG_CODE_ID \
  --node $NODE --chain-id $CHAIN_ID --home $HOME_DIR --output json 2>/dev/null | \
  python3 -c "import sys,json; c=json.load(sys.stdin).get('contracts',[]); print(c[-1] if c else '')" 2>/dev/null)

if [ -z "$AGG_ADDR" ]; then
  read -p "Enter aggregator contract address: " AGG_ADDR
fi
echo "Aggregator contract: $AGG_ADDR"

# ── Wire aggregator to CLSSC ──────────────────────────────────────────────────
echo ""
echo "=== Wiring aggregator to CLSSC ==="

# SetAggregator
$BINARY tx wasm execute $CLSSC_ADDR \
  "{\"set_aggregator\":{\"aggregator_addr\":\"$AGG_ADDR\"}}" \
  --from $FROM --chain-id $CHAIN_ID --node $NODE \
  --home $HOME_DIR $KR --gas 200000 --fees $FEES -y
sleep 6

echo "Aggregator wired. PoR NOT yet enforced — enable after feed is live:"
echo "  amisd tx wasm execute $CLSSC_ADDR '{\"set_por_enforced\":{\"enforced\":true}}' --from ops ..."

# ── Optional: deploy on-chain verifier ────────────────────────────────────────
if [ "$DEPLOY_ONCHAIN" = true ]; then
  echo ""
  echo "=== Building por_onchain ==="
  cd ../por_onchain
  bash ../../scripts/build_contract.sh
  ONCHAIN_WASM="artifacts/por_onchain.wasm"

  echo "=== Uploading por_onchain ==="
  OC_UPLOAD=$($BINARY tx wasm store $ONCHAIN_WASM \
    --from $FROM --chain-id $CHAIN_ID --node $NODE \
    --home $HOME_DIR $KR --gas 3000000 --fees $FEES -y --output json 2>&1)
  OC_TXHASH=$(echo $OC_UPLOAD | python3 -c "import sys,json; print(json.load(sys.stdin)['txhash'])" 2>/dev/null)
  sleep 8

  OC_CODE_ID=$($BINARY query tx $OC_TXHASH --node $NODE --chain-id $CHAIN_ID --home $HOME_DIR --output json 2>/dev/null | \
    python3 -c "
import sys,json
tx=json.load(sys.stdin)
for log in tx.get('logs',[]):
  for ev in log.get('events',[]):
    for attr in ev.get('attributes',[]):
      if attr.get('key')=='code_id': print(attr['value']); sys.exit(0)
" 2>/dev/null)

  if [ -z "$OC_CODE_ID" ]; then
    read -p "Enter por_onchain Code ID: " OC_CODE_ID
  fi

  echo "=== Instantiating por_onchain ==="
  echo "NOTE: Edit ISSUER_ADDR and TOKEN_CONTRACT before running in production"
  OC_INIT=$(cat <<EOF
{
  "admin": "$ADMIN",
  "aggregator_addr": "$AGG_ADDR",
  "issuer_addr": "$ADMIN",
  "description": "CLSSC on-chain USDC reserve verifier",
  "initial_sources": []
}
EOF
)
  $BINARY tx wasm instantiate $OC_CODE_ID "$OC_INIT" \
    --label "meridian-por-onchain-v1" --admin $ADMIN \
    --from $FROM --chain-id $CHAIN_ID --node $NODE \
    --home $HOME_DIR $KR --gas 500000 --fees $FEES -y
  sleep 8

  OC_ADDR=$($BINARY query wasm list-contract-by-code $OC_CODE_ID \
    --node $NODE --chain-id $CHAIN_ID --home $HOME_DIR --output json 2>/dev/null | \
    python3 -c "import sys,json; c=json.load(sys.stdin).get('contracts',[]); print(c[-1] if c else '')" 2>/dev/null)
  echo "On-chain verifier: $OC_ADDR"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  PoR DEPLOYMENT COMPLETE"
echo "═══════════════════════════════════════════════════════════════"
echo "  PoR Aggregator:      $AGG_ADDR"
echo "  CLSSC contract:      $CLSSC_ADDR"
echo "  Aggregator wired:    YES"
echo "  PoR enforced:        NO (enable manually when feed is live)"
if [ "$DEPLOY_ONCHAIN" = true ]; then
echo "  On-chain verifier:   $OC_ADDR"
fi
echo ""
echo "Next steps:"
echo "  1. Configure oracle-relayer/.env with AGGREGATOR_JOBS pointing to $AGG_ADDR"
echo "  2. Run: cd oracle-relayer && npm install && npm start"
echo "  3. Verify: curl http://localhost:3100/status"
echo "  4. After first successful submission, enable enforcement:"
echo "     amisd tx wasm execute $CLSSC_ADDR '{\"set_por_enforced\":{\"enforced\":true}}' --from ops --chain-id amitis-network --node https://rpc.amitis.network:443 --home ~/.amitis --keyring-backend test --gas 200000 --fees 150000uamts -y"
echo "  5. Test MintCheck:"
echo "     amisd query wasm contract-state smart $AGG_ADDR '{\"mint_check\":{\"mint_amount\":\"1000000\",\"circulating_supply\":\"0\"}}' --node https://rpc.amitis.network:443 --chain-id amitis-network --home ~/.amitis"
echo "═══════════════════════════════════════════════════════════════"
