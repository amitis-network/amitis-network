#!/usr/bin/env python3
# inject_genesis.py — Run after collect-gentxs
# Injects petals, community pool, bridge params, and chain params into genesis.json
# Usage: python3 inject_genesis.py ~/.amitis/config/genesis.json

import json
import sys

GENESIS_FILE = sys.argv[1] if len(sys.argv) > 1 else "~/.amitis/config/genesis.json"
GENESIS_FILE = GENESIS_FILE.replace("~", __import__("os").path.expanduser("~"))

print(f"Injecting genesis data into {GENESIS_FILE}...")

with open(GENESIS_FILE) as f:
    g = json.load(f)

# ── CHAIN PARAMS ──────────────────────────────────────────────────────────────
g["chain_id"] = "amitis-network"

# Staking params
g["app_state"]["staking"]["params"]["bond_denom"] = "uamts"
g["app_state"]["staking"]["params"]["unbonding_time"] = "1814400s"
g["app_state"]["staking"]["params"]["max_validators"] = 21

# Gov params
g["app_state"]["gov"]["params"]["voting_period"] = "86400s"
g["app_state"]["gov"]["params"]["expedited_voting_period"] = "43200s"
g["app_state"]["gov"]["params"]["min_deposit"] = [{"denom": "uamts", "amount": "10000000"}]

# Mint — disable inflation
g["app_state"]["mint"]["params"]["mint_denom"] = "uamts"
g["app_state"]["mint"]["params"]["inflation_rate_change"] = "0.000000000000000000"
g["app_state"]["mint"]["params"]["inflation_max"] = "0.000000000000000000"
g["app_state"]["mint"]["params"]["inflation_min"] = "0.000000000000000000"
g["app_state"]["mint"]["minter"]["inflation"] = "0.000000000000000000"

# Crisis
g["app_state"]["crisis"]["constant_fee"] = {"denom": "uamts", "amount": "1000"}

# Slashing
g["app_state"]["slashing"]["params"]["signed_blocks_window"] = "10000"
g["app_state"]["slashing"]["params"]["min_signed_per_window"] = "0.050000000000000000"
g["app_state"]["slashing"]["params"]["downtime_jail_duration"] = "60s"
g["app_state"]["slashing"]["params"]["slash_fraction_double_sign"] = "0.050000000000000000"
g["app_state"]["slashing"]["params"]["slash_fraction_downtime"] = "0.000000000000000000"

# Distribution
g["app_state"]["distribution"]["params"]["community_tax"] = "0.020000000000000000"

# ── COMMUNITY POOL ─────────────────────────────────────────────────────────────
# Set community pool via distribution fee_pool (not bank balances)
# The distribution module account handles this internally
community_pool_amount = "400000000000000"  # 400M AMTS
g["app_state"]["distribution"]["fee_pool"]["community_pool"] = [
    {"denom": "uamts", "amount": community_pool_amount}
]
# Add distribution module account to bank if not present
dist_addr = "amitis1jv65s3grqf6v6jl3dp4t6c9t9rk99cd8tl80w4"
existing = [b for b in g["app_state"]["bank"]["balances"] if b["address"] == dist_addr]
if not existing:
    g["app_state"]["bank"]["balances"].append({
        "address": dist_addr,
        "coins": [{"denom": "uamts", "amount": community_pool_amount}]
    })
    for s in g["app_state"]["bank"]["supply"]:
        if s["denom"] == "uamts":
            s["amount"] = str(int(s["amount"]) + int(community_pool_amount))
print("Community pool funded.")

# ── TOTAL SUPPLY CHECK ────────────────────────────────────────────────────────
total = sum(int(coin["amount"]) for b in g["app_state"]["bank"]["balances"] for coin in b["coins"] if coin["denom"] == "uamts")
print(f"Total uamts in balances: {total:,}")
expected = 1_000_000_000_000_000
if total != expected:
    print(f"WARNING: Expected {expected:,}, got {total:,} — difference: {total - expected:,}")
    print("Fixing supply...")
    for s in g["app_state"]["bank"]["supply"]:
        if s["denom"] == "uamts":
            s["amount"] = str(total)
    print(f"Supply set to {total:,}")
else:
    print("✓ Total supply correct: 1,000,000,000 AMTS")

# ── PETAL REGISTRY ────────────────────────────────────────────────────────────
petal_names = [
    "Rose", "Lotus", "Orchid", "Jasmine", "Lily",
    "Dahlia", "Iris", "Violet", "Marigold", "Tulip",
    "Magnolia", "Lavender", "Poppy", "Sunflower", "Daisy",
    "Peony", "Chrysanthemum", "Aster", "Cosmos", "Zinnia",
    "Amaranth"
]

# Validator operator addresses for petals 1-7 (active validators)
# Petals 6-21 owned by validator1 account (amitis14uxd...)
validator_ops = {
    1: {"owner": "amitis14uxdwplhhznclgaa350945tkyyw7e5e24zdfn6",  "val": "amitisvaloper14uxdwplhhznclgaa350945tkyyw7e5e2f5sam5"},
    2: {"owner": "amitis1f4wrlrjzyxeavzz9n5dfgx4d4n0tsg8yv6rga7",  "val": "amitisvaloper1f4wrlrjzyxeavzz9n5dfgx4d4n0tsg8ysv7u4s"},
    3: {"owner": "amitis14fmvjd4jmp3adgr97zqfnw3asgdr0efdnwkzxy", "val": "amitisvaloper14fmvjd4jmp3adgr97zqfnw3asgdr0efd0ctkw2"},
    4: {"owner": "amitis1sy2rqgl6ulvgtgp33qeeupvvunn8700luw0n5t",  "val": "amitisvaloper1sy2rqgl6ulvgtgp33qeeupvvunn8700lqcj8u9"},
    5: {"owner": "amitis12qywa3pksm9k2y6g70zsjatw6mejxfp26629e0", "val": "amitisvaloper12qywa3pksm9k2y6g70zsjatw6mejxfp2xvh33p"},
    6: {"owner": "amitis1srn92hlpgvwfy6h9yfs2h3ujaenh3fmgzzk25s",  "val": "amitisvaloper1srn92hlpgvwfy6h9yfs2h3ujaenh3fmg75t7u7"},
    7: {"owner": "amitis187l63xnptfhufjyuk9snj5panr9tunmmztwgaw",  "val": "amitisvaloper187l63xnptfhufjyuk9snj5panr9tunmm7anu4q"},
}

owner_for_remaining = "amitis14uxdwplhhznclgaa350945tkyyw7e5e24zdfn6"

petals = []
for i in range(1, 22):
    name = petal_names[i-1]
    if i in validator_ops:
        owner = validator_ops[i]["owner"]
        val_addr = validator_ops[i]["val"]
    else:
        owner = owner_for_remaining
        val_addr = ""
    petals.append({
        "id": str(i),
        "owner": owner,
        "validator_address": val_addr,
        "lifetime_revenue": "0",
        "purchase_price": {"denom": "uamts", "amount": "0"},
        "purchase_height": "0",
        "purchase_time": "0001-01-01T00:00:00Z",
        "name": name,
        "description": f"Amitis Network Petal #{i} - {name}"
    })

g["app_state"]["petalregistry"] = {"petals": petals}
print(f"✓ {len(petals)} petals injected into petalregistry.")

# ── BRIDGE MODULE GENESIS ──────────────────────────────────────────────────────
g["app_state"]["amitisbridge"] = {
    "params": {
        "attestation_threshold": 4,
        "request_expiry": 100,
        "bridge_fee_rate": "0.0005",
        "bridge_fee_rebate_rate": "0.80"
    },
    "inbound_requests": [],
    "outbound_requests": []
}
print("✓ amitisbridge genesis state injected.")

# ── WRITE OUTPUT ──────────────────────────────────────────────────────────────
with open(GENESIS_FILE, "w") as f:
    json.dump(g, f, indent=2)

print(f"\n✓ Genesis written to {GENESIS_FILE}")
print("Next: validate with: ~/amitis-network/build/amisd genesis validate --home ~/.amitis")
