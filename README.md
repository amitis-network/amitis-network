# Amitis Network

**Friendship Through Shared Prosperity**

Amitis Network is a cooperative Layer 1 blockchain built on the Cosmos SDK. The network returns 80% of every transaction fee directly to the user who paid it — hardcoded at the consensus layer, not adjustable by governance.

- **Chain ID**: `amitis-network`
- **RPC**: https://rpc.amitis.network
- **REST**: https://rest.amitis.network
- **Explorer**: https://app.amitis.network
- **Website**: https://amitis.network

---

## What Makes Amitis Different

### 80% Fee Rebate — Hardcoded at Consensus

Every transaction on Amitis returns 80% of the fee to the sender in the same block. This is implemented in the ante handler (`ante.go`) as `FeeRebateDecorator` — not a smart contract, not a governance parameter, not a reward program. It executes as part of consensus on every transaction.

```go
// ante.go — FeeRebateDecorator executes after DeductFeeDecorator
// 80% of every fee is returned to the transaction sender
// 20% is distributed equally across all 21 validator positions
```

### 21 Fixed Validator Positions

The network has exactly 21 consensus validator slots — permanently. Each position (called a "petal") is registered on-chain in the `x/petalregistry` module and is a transferable operating license. There will never be a 22nd validator.

### Protocol-Level DEX and Flash Loans

The native DEX (`x/amitisdex`) and flash loan facility (`x/samts`) are built into the protocol — not smart contracts. Every swap and flash loan generates fees that flow through the same 80/20 rebate mechanic as standard transactions.

- Flash loan fee: 0.05% (vs 0.09% on Aave)
- Flash loan pool: 21,000,000 AMTS backed by validator stake
- DEX: Constant product AMM at the consensus layer

### No Founder Token Allocation

The founder received zero AMTS at genesis. There is no team allocation, no VC vesting, no scheduled selling pressure. The full genesis distribution is on-chain and publicly verifiable.

### IBC Connected

Active IBC channel to Osmosis:
- Amitis: `channel-2` / `07-tendermint-1`
- Osmosis: `channel-110342` / `07-tendermint-3713`

### CosmWasm Enabled

Third-party developers can deploy CosmWasm smart contracts on Amitis. All contract interactions benefit from the 80% fee rebate automatically.

---

## Custom Modules

| Module | Path | Description |
|--------|------|-------------|
| `x/cooperative` | `x/cooperative/` | 80% fee rebate hardcoded at consensus |
| `x/petalregistry` | `x/petalregistry/` | 21 fixed validator position registry |
| `x/amitisdex` | `x/amitisdex/` | Protocol-level AMM DEX |
| `x/samts` | `x/samts/` | Flash loan facility backed by validator stake |
| `x/amitisbridge` | `x/amitisbridge/` | Cross-chain bridge module (in development) |

---

## Building

> **Important**: Always build with `-tags app_v1`. Omitting this tag silently compiles `app_v2.go` which excludes all custom modules.

```bash
git clone https://github.com/amitis-network/amitis-network
cd amitis-network
go build -tags app_v1 -o build/amisd ./simd/main.go
```

**Requirements:**
- Go 1.21+
- `libwasmvm.x86_64.so` (from wasmvm v2.1.2) must be present at `/lib/libwasmvm.x86_64.so` on the host

---

## Running a Node

```bash
# Initialize
./build/amisd init <moniker> --chain-id amitis-network --home ~/.amitis

# Download genesis
curl -s https://rpc.amitis.network/genesis | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin)['result']['genesis'], indent=2))" > ~/.amitis/config/genesis.json

# Configure peers
# Add to ~/.amitis/config/config.toml:
# persistent_peers = "2e6d2c3bca652e57c7192ef144223bb3a765eb91@rpc.amitis.network:26656"

# Start
./build/amisd start --home ~/.amitis
```

### State Sync (Recommended)

State sync allows a new node to bootstrap in minutes rather than replaying the full chain history:

```toml
# config.toml [statesync] section
enable = true
rpc_servers = "https://rpc.amitis.network:443,https://rpc.amitis.network:443"
trust_height = <recent_height>
trust_hash = "<hash_at_trust_height>"
trust_period = "2000h0m0s"
```

Get the trust hash:
```bash
curl -s "https://rpc.amitis.network/block?height=<trust_height>" | python3 -c "import sys,json; print(json.load(sys.stdin)['result']['block_id']['hash'])"
```

---

## Deploying CosmWasm Contracts

Amitis supports CosmWasm smart contracts. Contracts must be built with `wasm-opt` using `--llvm-memory-copy-fill-lowering` due to wasmvm v2.1.2's bulk memory requirements:

```bash
# Build contract
cd contracts/your_contract
cargo build --release --target wasm32-unknown-unknown

# Optimize (required)
wasm-opt -Os --llvm-memory-copy-fill-lowering \
  target/wasm32-unknown-unknown/release/your_contract.wasm \
  -o artifacts/your_contract.wasm

# Deploy
amisd tx wasm store artifacts/your_contract.wasm \
  --from <key> \
  --chain-id amitis-network \
  --node https://rpc.amitis.network \
  --fees <amount>uamts \
  -y
```

---

## Deployed Contracts

| Contract | Code ID | Address |
|----------|---------|---------|
| VPT Token | 1 | `amitis14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s5cvps3` |
| VPT/AMTS Pool | 3 | `amitis1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3smn7358` |
| AmitisSwap | 7 | `amitis1f6jlx7d9y408tlzue7r2qcf79plp549n30yzqjajjud8vm7m4vdspgrfqp` |

---

## API Reference

All endpoints available at `https://rest.amitis.network`:

```bash
# Network status
GET /cosmos/base/tendermint/v1beta1/node_info

# Validator set
GET /cosmos/staking/v1beta1/validators

# Petal registry (custom)
GET /amitis/petalregistry/v1/petals
GET /amitis/petalregistry/v1/petals/{id}

# Flash loan pool (custom)
GET /amitis/samts/v1/pool

# IBC channels
GET /ibc/core/channel/v1/channels

# Account balances
GET /cosmos/bank/v1beta1/balances/{address}
```

---

## Token Information

| Property | Value |
|----------|-------|
| Name | AMTS |
| Denom | `uamts` (1 AMTS = 1,000,000 uamts) |
| Total Supply | 1,000,000,000 AMTS (fixed, no inflation) |
| Founder Allocation | 0 AMTS |
| VC Allocation | 0 AMTS |

---

## Community

- **Website**: https://amitis.network
- **X**: [@AmitisNetwork](https://x.com/AmitisNetwork)
- **Discord**: https://discord.gg/RfNUWhYMd
- **Explorer**: https://app.amitis.network

---

## License

Apache 2.0 — see [LICENSE](LICENSE)

