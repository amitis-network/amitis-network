#!/bin/bash
# generate_genesis_petals.sh
# Injects 21 named petals into genesis.json
# Usage: ./generate_genesis_petals.sh ~/.amitis/config/genesis.json

GENESIS=${1:-~/.amitis/config/genesis.json}

echo "Injecting 21 petals into $GENESIS..."

python3 - "$GENESIS" << 'PYEOF'
import json, sys

genesis_file = sys.argv[1]

petal_names = [
    "Rose", "Lotus", "Orchid", "Jasmine", "Lily",
    "Dahlia", "Iris", "Violet", "Marigold", "Tulip",
    "Magnolia", "Lavender", "Poppy", "Sunflower", "Daisy",
    "Peony", "Chrysanthemum", "Aster", "Cosmos", "Zinnia",
    "Amaranth"
]

validator_addrs = [
    "amitisvaloper14uxdwplhhznclgaa350945tkyyw7e5e2f5sam5",
    "amitisvaloper1f4wrlrjzyxeavzz9n5dfgx4d4n0tsg8ysv7u4s",
    "amitisvaloper14fmvjd4jmp3adgr97zqfnw3asgdr0efd0ctkw2",
    "amitisvaloper1sy2rqgl6ulvgtgp33qeeupvvunn8700lqcj8u9",
    "amitisvaloper12qywa3pksm9k2y6g70zsjatw6mejxfp2xvh33p",
]

owner_addrs = [
    "amitis14uxdwplhhznclgaa350945tkyyw7e5e24zdfn6",
    "amitis1f4wrlrjzyxeavzz9n5dfgx4d4n0tsg8yv6rga7",
    "amitis14fmvjd4jmp3adgr97zqfnw3asgdr0efdnwkzxy",
    "amitis1sy2rqgl6ulvgtgp33qeeupvvunn8700luw0n5t",
    "amitis12qywa3pksm9k2y6g70zsjatw6mejxfp26629e0",
]

with open(genesis_file, 'r') as f:
    genesis = json.load(f)

petals = []
for i in range(1, 22):
    idx = i - 1
    owner = owner_addrs[idx] if idx < 5 else owner_addrs[0]
    val_addr = validator_addrs[idx] if idx < 5 else ""
    petals.append({
        "id": str(i),
        "owner": owner,
        "validator_address": val_addr,
        "lifetime_revenue": "0",
        "purchase_price": {"denom": "uamts", "amount": "0"},
        "purchase_height": "0",
        "purchase_time": "0001-01-01T00:00:00Z",
        "name": petal_names[idx],
        "description": f"Amitis Network Petal #{i} - {petal_names[idx]}"
    })

genesis['app_state']['petalregistry'] = {
    'petals': petals,
    'next_petal_id': '22'
}

genesis['app_state']['cooperative'] = {
    'total_rebates_paid': '0'
}

with open(genesis_file, 'w') as f:
    json.dump(genesis, f, indent=2)

print(f"Injected {len(petals)} petals into genesis")
print(f"Initialized cooperative module state")
PYEOF
