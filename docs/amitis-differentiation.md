# Amitis Network: A Different Kind of Layer 1

*"Friendship through shared prosperity."*

---

## The Problem With Most Layer 1 Blockchains

Every major Layer 1 blockchain launched with the same playbook: founders take a significant token allocation at genesis, investors get preferential access, validators are commoditized infrastructure, and users pay fees into a black hole they never see again. The people who build the network get rich. The people who use it subsidize that wealth.

Amitis Network was designed from first principles to be different — not as a marketing pitch, but as a set of hard protocol-level commitments that cannot be changed without governance.

---

## 1. The Founder Takes No Genesis Allocation

In most Layer 1 launches, the founding team allocates themselves 10–20% of the total token supply at genesis. On a 1B token supply, that's 100–200 million tokens handed to a small group before the network has done anything.

**On Amitis, the founder takes no genesis allocation.**

The founder participates in the network exactly like every other validator operator — by running a validator node, earning a petal (see below), and receiving their share of the 20% validator fee distribution. If the network succeeds, the founder benefits proportionally to their contribution, not because they printed tokens for themselves at launch.

This is a meaningful commitment. It means the founder's incentives are permanently aligned with the network's health, not with token price manipulation or early exits.

**Token distribution at genesis:**
- 21,000,000 AMTS — 21 validator operators (1M each, working capital)
- 400,000,000 AMTS — Community Pool (governance-controlled)
- 200,000,000 AMTS — Ecosystem Fund (DEX liquidity, bridge collateral, integrations)
- 200,000,000 AMTS — Staking Rewards Reserve (early staker incentives)
- 129,000,000 AMTS — Public Sale Reserve (future fundraising, governance-unlocked)
- 50,000,000 AMTS — Utility Reserve (operational, multi-sig controlled)

No individual wallet. No team allocation. No vesting cliff that unlocks a founder dump.

---

## 2. Validators Are Businesses, Not Commodity Infrastructure

On most blockchains, validators are interchangeable. Anyone can spin one up, anyone can get slashed out, and the set changes constantly. There's no intrinsic value in being a validator beyond the yield — and yield compresses as competition increases.

**Amitis introduces the Petal — a fixed, on-chain validator position that is a tradeable business asset.**

There are exactly 21 petals. Each petal corresponds to one validator slot. Petals are registered on-chain in the `petalregistry` module with an owner address, a validator address, and a growing lifetime revenue record. They can be transferred between owners via on-chain transactions.

What this means in practice:

- **Scarcity**: There will never be more than 21 validator positions. Supply is permanently fixed at genesis.
- **Transferable value**: A petal representing a validator that has processed billions in transactions has a verifiable on-chain revenue history. It can be priced and sold.
- **Growing valuation**: As network usage grows, fee revenue flows to petal owners. A petal purchased early, when fees are low, appreciates as the network scales — exactly like owning equity in a profitable business.
- **Separation of ownership and operation**: A petal owner doesn't have to run the validator themselves. They can delegate operation to a technical partner while retaining ownership of the economic position.

This creates a fundamentally different validator economics model. Validators aren't just infrastructure — they're businesses with book value, revenue history, and transferable ownership.

---

## 3. 80% of Transaction Fees Go Back to Users

Most blockchains collect transaction fees and distribute them to validators and stakers. Users pay and get nothing back. On Ethereum, base fees are burned (benefiting holders), but the user still loses the fee.

**Amitis returns 80% of every transaction fee to the sender, automatically, at the protocol level.**

This is implemented in the `cooperative` module via an ante handler decorator — meaning it happens at the consensus layer, not as a smart contract that can be upgraded or paused. Every transaction, every time, 80% back.

The remaining 20% is distributed to the 21 validator petals proportionally.

The math works because validators benefit from volume rather than per-transaction extraction. A high-throughput network with 80% rebates generates more validator revenue than a low-throughput network with 100% fee capture.

For users, this changes the economics of on-chain activity fundamentally. Heavy users of the network — traders, protocols, bridges — recoup most of their transaction costs. The network rewards participation rather than taxing it.

---

## 4. DEX and Bridge at the Protocol Layer

On most blockchains, DEXes and bridges are smart contracts — third-party applications deployed on top of the chain. This creates several problems:

- **Smart contract risk**: Bugs in contract code can drain liquidity (see: dozens of DeFi exploits)
- **Fee extraction**: The DEX takes fees on top of the chain's transaction fees
- **Fragmentation**: Multiple competing DEXes split liquidity
- **Upgrade risk**: Contract upgrades can change rules, rug liquidity, or introduce vulnerabilities

**Amitis builds the DEX and bridge directly into the protocol as native modules.**

The `amitisdex` module implements an AMM (automated market maker) as a first-class blockchain module — not a smart contract. Liquidity pools are state in the chain's KV store. Swaps are transactions processed by the validator set. There is no contract to exploit.

The `amitisbridge` module provides cross-chain asset transfers at the protocol level, with the same security guarantees as any other chain transaction.

This means:
- No smart contract risk for core DeFi primitives
- Swap fees benefit the network (subject to the 80% rebate)
- A single canonical DEX with unified liquidity
- Bridge security backed by the full validator set, not a multisig

---

## 5. Liquid Staking as a Native Feature

Most liquid staking on other chains is provided by third-party protocols (Lido, Rocket Pool, etc.) that introduce their own governance tokens, smart contract risk, and fee extraction.

The `liquidstaking` module on Amitis is part of the core protocol. Stakers receive liquid staking tokens representing their bonded position, usable in the native DEX and bridge, without trusting any third-party protocol.

---

## The Cooperative Model

The name "Amitis" comes from an ancient Persian word for friendship. The network's tagline — "Friendship through shared prosperity" — is a design constraint, not a marketing slogan.

Every major architectural decision flows from one question: *does this distribute value to participants, or extract it from them?*

- No founder allocation → value stays in the network
- 80% fee rebate → value flows back to users
- Fixed petal validators → value accrues to long-term operators
- Native DEX/bridge → value stays in the protocol, not third parties
- Governance-controlled reserves → value distributed by community vote

The result is a blockchain where the incentives of founders, validators, and users are genuinely aligned — not through clever tokenomics designed to look aligned while hiding extraction, but through hard protocol rules that make extraction structurally impossible.

---

## Technical Foundation

- **Consensus**: CometBFT (Tendermint) with instant finality
- **SDK**: Cosmos SDK v0.50.6
- **Validator Set**: 21 fixed positions (petals)
- **Block Time**: ~5 seconds
- **Native Token**: AMTS (1B total supply, fixed)
- **Address Prefix**: `amitis`
- **IBC Compatible**: Yes (Cosmos ecosystem interoperability)
- **Fee Rebate**: 80% at protocol level via ante handler
- **Modules**: petalregistry, cooperative, amitisdex, liquidstaking, amitisbridge

---

*Amitis Network is currently in development with a 7-validator testnet running. The full 21-validator mainnet will launch with the complete module suite.*
