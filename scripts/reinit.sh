#!/bin/bash
# reinit.sh — Full Amitis Network genesis reinit
# Run from .50 as amitis user
# All validators must be stopped before running

set -e

BINARY=~/amitis-network/build/amisd
HOME_DIR=~/.amitis
CHAIN_ID="amitis-network"

echo "=== Amitis Network Reinit ==="
echo "Binary: $BINARY"
echo "Home: $HOME_DIR"
echo "Chain ID: $CHAIN_ID"
echo ""
echo "WARNING: This will wipe all chain data. All validators must be stopped."
read -p "Type YES to continue: " confirm
if [ "$confirm" != "YES" ]; then
    echo "Aborted."
    exit 1
fi

# ── WIPE ──────────────────────────────────────────────────────────────────────
echo ""
echo "Wiping existing data..."
rm -rf $HOME_DIR

# ── INIT ──────────────────────────────────────────────────────────────────────
echo "Initializing chain..."
$BINARY init amitis-validator-1 --chain-id $CHAIN_ID --home $HOME_DIR

# ── RESTORE KEYRING ───────────────────────────────────────────────────────────
echo "Restoring keyring from backup..."
cp -r ~/amitis-keyring-backup/* $HOME_DIR/keyring-test/

# ── ADD GENESIS ACCOUNTS ──────────────────────────────────────────────────────
echo "Adding genesis accounts..."

# Validator accounts (1M AMTS each = 1000000000000 uamts)
$BINARY genesis add-genesis-account amitis14uxdwplhhznclgaa350945tkyyw7e5e24zdfn6 1000000000000uamts --home $HOME_DIR
$BINARY genesis add-genesis-account amitis1f4wrlrjzyxeavzz9n5dfgx4d4n0tsg8yv6rga7 1000000000000uamts --home $HOME_DIR
$BINARY genesis add-genesis-account amitis14fmvjd4jmp3adgr97zqfnw3asgdr0efdnwkzxy 1000000000000uamts --home $HOME_DIR
$BINARY genesis add-genesis-account amitis1sy2rqgl6ulvgtgp33qeeupvvunn8700luw0n5t 1000000000000uamts --home $HOME_DIR
$BINARY genesis add-genesis-account amitis12qywa3pksm9k2y6g70zsjatw6mejxfp26629e0 1000000000000uamts --home $HOME_DIR
$BINARY genesis add-genesis-account amitis1srn92hlpgvwfy6h9yfs2h3ujaenh3fmgzzk25s 1000000000000uamts --home $HOME_DIR
$BINARY genesis add-genesis-account amitis187l63xnptfhufjyuk9snj5panr9tunmmztwgaw 1000000000000uamts --home $HOME_DIR

# Reserve accounts
$BINARY genesis add-genesis-account amitis1clgvqc03f8atzn5z00x8jzx9nl4sv4q3dul5lu 200000000000000uamts --home $HOME_DIR  # ecosystem-fund
$BINARY genesis add-genesis-account amitis1cj9dszt7f3vmqz6aps2h832y6sqhycf30vfrvh 200000000000000uamts --home $HOME_DIR  # staking-reserve
$BINARY genesis add-genesis-account amitis1al0f6wld6jetqu973560te73jk0c40qnfkmmpp 129000000000000uamts --home $HOME_DIR  # public-sale-reserve
$BINARY genesis add-genesis-account amitis1hqay4g2g2w0ck2ul43ekvpwgvt9xttlmdnexpe 64000000000000uamts --home $HOME_DIR   # utility-reserve (50M + 14M to make total 1B)

echo "Genesis accounts added."

# ── GENTX FOR VALIDATOR 1 (local) ────────────────────────────────────────────
echo "Creating gentx for validator-1..."
$BINARY genesis gentx validator1 1000000000000uamts \
    --chain-id $CHAIN_ID \
    --moniker "amitis-validator-1" \
    --keyring-backend test \
    --keyring-dir $HOME_DIR \
    --home $HOME_DIR

# ── ENABLE REST API ────────────────────────────────────────────────────────────
echo "Enabling REST API..."
sed -i '0,/enable = false/s/enable = false/enable = true/' $HOME_DIR/config/app.toml

# ── ENABLE RPC ON ALL INTERFACES ──────────────────────────────────────────────
echo "Enabling RPC on 0.0.0.0..."
sed -i 's|laddr = "tcp://127.0.0.1:26657"|laddr = "tcp://0.0.0.0:26657"|' $HOME_DIR/config/config.toml

# ── ENABLE CORS ────────────────────────────────────────────────────────────────
echo "Enabling CORS..."
sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/' $HOME_DIR/config/config.toml

echo "Gentx created. Now copy gentx to other validators and collect."
echo ""
echo "Next steps:"
echo "1. Copy $HOME_DIR/config/genesis.json to all other validators"
echo "2. On each validator run:"
echo "   ~/amitis-network/build/amisd genesis gentx validatorX 1000000000000uamts --chain-id amitis-network --moniker amitis-validator-X --keyring-backend test --home ~/.amitis"
echo "3. Copy all gentx JSON files to $HOME_DIR/config/gentx/ on .50"
echo "4. Run: $BINARY genesis collect-gentxs --home $HOME_DIR"
echo "5. Run: ./inject_genesis.sh (to add petals, community pool, bridge params)"
