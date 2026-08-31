package types

const (
	ModuleName = "amitisbridge"
	StoreKey   = "amitisbridge"
	RouterKey  = "amitisbridge"

	// Supported source chains
	ChainEthereum = "ethereum"
	ChainBSC      = "bsc"
	ChainPolygon  = "polygon"
	ChainArbitrum = "arbitrum"
	ChainOptimism = "optimism"

	// Bridge request statuses
	StatusPending  = "pending"
	StatusComplete = "complete"
	StatusExpired  = "expired"

	// Default params
	DefaultAttestationThreshold = 4   // 4-of-7 initially; raise to 11 via governance at 21 validators
	DefaultRequestExpiry        = 100 // blocks before a pending request expires
	DefaultBridgeFeeRate        = "0.0005" // 0.05%
	DefaultBridgeFeeRebateRate  = "0.80"   // 80% rebated to user

	// Denom mapping — wrapped assets use u prefix consistent with uamts
	DenomWrappedETH   = "uweth"
	DenomWrappedUSDC  = "uusdc"
	DenomWrappedBNB   = "ubnb"
	DenomWrappedMATIC = "uwmatic"
	DenomWrappedARB   = "uwarb"
	DenomWrappedOP    = "uwop"
	DenomWrappedWBTC  = "uwbtc"
)

var (
	// KV store key prefixes
	KeyInboundRequest  = []byte{0x01} // inbound: EVM -> Amitis (mint)
	KeyOutboundRequest = []byte{0x02} // outbound: Amitis -> EVM (burn + release)
	KeyAttestation     = []byte{0x03} // attestation records per request
	KeyParams          = []byte{0x04} // module params
	KeyNonce           = []byte{0x05} // per-chain nonce tracker
)

// DenomForChainToken returns the Amitis denom for a given chain+token pair
func DenomForChainToken(chain, token string) string {
	switch token {
	case "ETH", "eth":
		return DenomWrappedETH
	case "USDC", "usdc":
		return DenomWrappedUSDC
	case "BNB", "bnb":
		return DenomWrappedBNB
	case "MATIC", "matic":
		return DenomWrappedMATIC
	case "ARB", "arb":
		return DenomWrappedARB
	case "OP", "op":
		return DenomWrappedOP
	case "WBTC", "wbtc":
		return DenomWrappedWBTC
	default:
		return "u" + token
	}
}
