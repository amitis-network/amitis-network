package types

import (
	"fmt"

	sdk "github.com/cosmos/cosmos-sdk/types"
)

// ── PARAMS ────────────────────────────────────────────────────────────────────

type Params struct {
	AttestationThreshold int    `json:"attestation_threshold"`
	RequestExpiry        int64  `json:"request_expiry"`
	BridgeFeeRate        string `json:"bridge_fee_rate"`
	BridgeFeeRebateRate  string `json:"bridge_fee_rebate_rate"`
}

func DefaultParams() Params {
	return Params{
		AttestationThreshold: DefaultAttestationThreshold,
		RequestExpiry:        DefaultRequestExpiry,
		BridgeFeeRate:        DefaultBridgeFeeRate,
		BridgeFeeRebateRate:  DefaultBridgeFeeRebateRate,
	}
}

// ── BRIDGE REQUEST ────────────────────────────────────────────────────────────

// InboundRequest represents an EVM → Amitis bridge request.
// Created when the first validator attests a lock event on the source chain.
type InboundRequest struct {
	ID             string `json:"id"`              // sha256(chain+txHash+logIndex)
	SourceChain    string `json:"source_chain"`    // "ethereum", "bsc", etc.
	SourceTxHash   string `json:"source_tx_hash"`  // EVM transaction hash (0x...)
	SourceLogIndex uint64 `json:"source_log_index"`
	Sender         string `json:"sender"`          // EVM sender address (0x...)
	Recipient      string `json:"recipient"`       // amitis1... destination
	Token          string `json:"token"`           // "ETH", "USDC", etc.
	Amount         string `json:"amount"`          // in smallest unit (wei for ETH)
	AmitisAmount   string `json:"amitis_amount"`   // converted to uamts decimals
	AmitisDenom    string `json:"amitis_denom"`    // "uweth", "uusdc", etc.
	Status         string `json:"status"`          // pending / complete / expired
	Attestations   int    `json:"attestations"`    // count of validator attestations
	CreatedHeight  int64  `json:"created_height"`
	CompletedHeight int64 `json:"completed_height,omitempty"`
}

// OutboundRequest represents an Amitis → EVM withdrawal request.
// Created when a user burns wrapped assets on Amitis.
type OutboundRequest struct {
	ID              string `json:"id"`
	DestChain       string `json:"dest_chain"`
	DestAddress     string `json:"dest_address"`  // EVM 0x address
	Sender          string `json:"sender"`        // amitis1... address
	AmitisDenom     string `json:"amitis_denom"`
	AmitisAmount    string `json:"amitis_amount"`
	Token           string `json:"token"`
	Amount          string `json:"amount"`        // EVM amount
	Status          string `json:"status"`
	Attestations    int    `json:"attestations"`
	CreatedHeight   int64  `json:"created_height"`
	CompletedHeight int64  `json:"completed_height,omitempty"`
}

// AttestationRecord tracks which validators have attested a given request
type AttestationRecord struct {
	RequestID  string   `json:"request_id"`
	Validators []string `json:"validators"` // validator operator addresses
}

// ── GENESIS ───────────────────────────────────────────────────────────────────

type GenesisState struct {
	Params            Params            `json:"params"`
	InboundRequests   []InboundRequest  `json:"inbound_requests"`
	OutboundRequests  []OutboundRequest `json:"outbound_requests"`
}

func DefaultGenesisState() GenesisState {
	return GenesisState{
		Params:           DefaultParams(),
		InboundRequests:  []InboundRequest{},
		OutboundRequests: []OutboundRequest{},
	}
}

// ── MESSAGES ──────────────────────────────────────────────────────────────────

// MsgSubmitAttestation is sent by a validator to attest an EVM lock event.
// When attestation count reaches threshold, tokens are minted to recipient.
type MsgSubmitAttestation struct {
	Validator      string `json:"validator"`       // amitisvaloper1...
	SourceChain    string `json:"source_chain"`
	SourceTxHash   string `json:"source_tx_hash"`
	SourceLogIndex uint64 `json:"source_log_index"`
	Sender         string `json:"sender"`
	Recipient      string `json:"recipient"`
	Token          string `json:"token"`
	Amount         string `json:"amount"`
}

func (msg MsgSubmitAttestation) ValidateBasic() error {
	if msg.Validator == "" {
		return fmt.Errorf("validator address required")
	}
	if msg.SourceTxHash == "" {
		return fmt.Errorf("source tx hash required")
	}
	if msg.Recipient == "" {
		return fmt.Errorf("recipient required")
	}
	if _, err := sdk.AccAddressFromBech32(msg.Recipient); err != nil {
		return fmt.Errorf("invalid recipient address: %w", err)
	}
	if msg.Amount == "" || msg.Amount == "0" {
		return fmt.Errorf("amount must be positive")
	}
	return nil
}

type MsgSubmitAttestationResponse struct {
	RequestID string `json:"request_id"`
	Status    string `json:"status"` // "attested" or "completed" (threshold reached)
	Minted    bool   `json:"minted"`
}

// MsgInitiateWithdrawal is sent by a user to burn wrapped assets and
// request release of the original asset on the destination EVM chain.
type MsgInitiateWithdrawal struct {
	Sender      string `json:"sender"`       // amitis1...
	DestChain   string `json:"dest_chain"`
	DestAddress string `json:"dest_address"` // 0x...
	Denom       string `json:"denom"`        // "uweth", "uusdc", etc.
	Amount      string `json:"amount"`       // in udenom units
}

func (msg MsgInitiateWithdrawal) ValidateBasic() error {
	if _, err := sdk.AccAddressFromBech32(msg.Sender); err != nil {
		return fmt.Errorf("invalid sender: %w", err)
	}
	if msg.DestAddress == "" {
		return fmt.Errorf("destination address required")
	}
	if len(msg.DestAddress) < 40 {
		return fmt.Errorf("invalid destination address")
	}
	if msg.Denom == "" {
		return fmt.Errorf("denom required")
	}
	if msg.Amount == "" || msg.Amount == "0" {
		return fmt.Errorf("amount must be positive")
	}
	switch msg.DestChain {
	case ChainEthereum, ChainBSC, ChainPolygon, ChainArbitrum, ChainOptimism:
	default:
		return fmt.Errorf("unsupported destination chain: %s", msg.DestChain)
	}
	return nil
}

type MsgInitiateWithdrawalResponse struct {
	RequestID string `json:"request_id"`
	Status    string `json:"status"`
}

// MsgUpdateBridgeParams is sent by governance to update bridge parameters.
type MsgUpdateBridgeParams struct {
	Authority string `json:"authority"`
	Params    Params `json:"params"`
}

func (msg MsgUpdateBridgeParams) ValidateBasic() error {
	if msg.Authority == "" {
		return fmt.Errorf("authority required")
	}
	if msg.Params.AttestationThreshold < 1 {
		return fmt.Errorf("attestation threshold must be >= 1")
	}
	return nil
}

type MsgUpdateBridgeParamsResponse struct{}

// ── INTERFACE STUBS (no protobuf — direct registration) ──────────────────────

type MsgServer interface {
	SubmitAttestation(ctx interface{}, msg *MsgSubmitAttestation) (*MsgSubmitAttestationResponse, error)
	InitiateWithdrawal(ctx interface{}, msg *MsgInitiateWithdrawal) (*MsgInitiateWithdrawalResponse, error)
	UpdateBridgeParams(ctx interface{}, msg *MsgUpdateBridgeParams) (*MsgUpdateBridgeParamsResponse, error)
}
