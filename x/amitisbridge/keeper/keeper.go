package keeper

import (
	"context"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"math/big"

	"cosmossdk.io/core/store"
	sdkmath "cosmossdk.io/math"
	sdk "github.com/cosmos/cosmos-sdk/types"
	bankkeeper "github.com/cosmos/cosmos-sdk/x/bank/keeper"
	authtypes "github.com/cosmos/cosmos-sdk/x/auth/types"

	"github.com/amitis-network/amitis-network/x/amitisbridge/types"
)

type Keeper struct {
	storeService store.KVStoreService
	BankKeeper   bankkeeper.Keeper
	FeeCollector string
}

func NewKeeper(
	storeService store.KVStoreService,
	bankKeeper bankkeeper.Keeper,
) Keeper {
	return Keeper{
		storeService: storeService,
		BankKeeper:   bankKeeper,
		FeeCollector: authtypes.FeeCollectorName,
	}
}

// ── PARAMS ────────────────────────────────────────────────────────────────────

func (k Keeper) GetParams(ctx context.Context) types.Params {
	s := k.storeService.OpenKVStore(ctx)
	bz, err := s.Get(types.KeyParams)
	if err != nil || bz == nil {
		return types.DefaultParams()
	}
	var p types.Params
	_ = json.Unmarshal(bz, &p)
	return p
}

func (k Keeper) SetParams(ctx context.Context, p types.Params) error {
	s := k.storeService.OpenKVStore(ctx)
	bz, err := json.Marshal(p)
	if err != nil {
		return err
	}
	return s.Set(types.KeyParams, bz)
}

// ── REQUEST ID ────────────────────────────────────────────────────────────────

func RequestID(chain, txHash string, logIndex uint64) string {
	h := sha256.New()
	h.Write([]byte(fmt.Sprintf("%s:%s:%d", chain, txHash, logIndex)))
	return fmt.Sprintf("%x", h.Sum(nil))[:32]
}

// ── INBOUND REQUESTS ──────────────────────────────────────────────────────────

func (k Keeper) SetInboundRequest(ctx context.Context, req types.InboundRequest) error {
	s := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyInboundRequest, []byte(req.ID)...)
	bz, err := json.Marshal(req)
	if err != nil {
		return err
	}
	return s.Set(key, bz)
}

func (k Keeper) GetInboundRequest(ctx context.Context, id string) (types.InboundRequest, error) {
	s := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyInboundRequest, []byte(id)...)
	bz, err := s.Get(key)
	if err != nil || bz == nil {
		return types.InboundRequest{}, fmt.Errorf("inbound request not found: %s", id)
	}
	var req types.InboundRequest
	return req, json.Unmarshal(bz, &req)
}

func (k Keeper) GetAllInboundRequests(ctx context.Context) ([]types.InboundRequest, error) {
	s := k.storeService.OpenKVStore(ctx)
	iter, err := s.Iterator(types.KeyInboundRequest, append(types.KeyInboundRequest, 0xFF))
	if err != nil {
		return nil, err
	}
	defer iter.Close()
	var reqs []types.InboundRequest
	for ; iter.Valid(); iter.Next() {
		var req types.InboundRequest
		if err := json.Unmarshal(iter.Value(), &req); err != nil {
			continue
		}
		reqs = append(reqs, req)
	}
	return reqs, nil
}

// ── OUTBOUND REQUESTS ─────────────────────────────────────────────────────────

func (k Keeper) SetOutboundRequest(ctx context.Context, req types.OutboundRequest) error {
	s := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyOutboundRequest, []byte(req.ID)...)
	bz, err := json.Marshal(req)
	if err != nil {
		return err
	}
	return s.Set(key, bz)
}

func (k Keeper) GetOutboundRequest(ctx context.Context, id string) (types.OutboundRequest, error) {
	s := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyOutboundRequest, []byte(id)...)
	bz, err := s.Get(key)
	if err != nil || bz == nil {
		return types.OutboundRequest{}, fmt.Errorf("outbound request not found: %s", id)
	}
	var req types.OutboundRequest
	return req, json.Unmarshal(bz, &req)
}

func (k Keeper) GetAllOutboundRequests(ctx context.Context) ([]types.OutboundRequest, error) {
	s := k.storeService.OpenKVStore(ctx)
	iter, err := s.Iterator(types.KeyOutboundRequest, append(types.KeyOutboundRequest, 0xFF))
	if err != nil {
		return nil, err
	}
	defer iter.Close()
	var reqs []types.OutboundRequest
	for ; iter.Valid(); iter.Next() {
		var req types.OutboundRequest
		if err := json.Unmarshal(iter.Value(), &req); err != nil {
			continue
		}
		reqs = append(reqs, req)
	}
	return reqs, nil
}

// ── ATTESTATIONS ──────────────────────────────────────────────────────────────

func (k Keeper) GetAttestation(ctx context.Context, requestID string) (types.AttestationRecord, error) {
	s := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyAttestation, []byte(requestID)...)
	bz, err := s.Get(key)
	if err != nil || bz == nil {
		return types.AttestationRecord{RequestID: requestID, Validators: []string{}}, nil
	}
	var rec types.AttestationRecord
	return rec, json.Unmarshal(bz, &rec)
}

func (k Keeper) AddAttestation(ctx context.Context, requestID, validator string) (int, error) {
	rec, err := k.GetAttestation(ctx, requestID)
	if err != nil {
		return 0, err
	}
	// Check for duplicate attestation
	for _, v := range rec.Validators {
		if v == validator {
			return len(rec.Validators), fmt.Errorf("validator %s already attested request %s", validator, requestID)
		}
	}
	rec.Validators = append(rec.Validators, validator)
	s := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyAttestation, []byte(requestID)...)
	bz, err := json.Marshal(rec)
	if err != nil {
		return 0, err
	}
	return len(rec.Validators), s.Set(key, bz)
}

// ── INBOUND EXECUTION (mint on threshold) ────────────────────────────────────

// convertAmount converts EVM wei/base units to uamts-scale (6 decimals)
// ETH: 18 decimals → divide by 1e12 to get 6 decimal representation
// USDC: 6 decimals on EVM → 1:1
func convertAmount(token, amount string) (string, error) {
	n := new(big.Int)
	if _, ok := n.SetString(amount, 10); !ok {
		return "", fmt.Errorf("invalid amount: %s", amount)
	}
	switch token {
	case "ETH", "WBTC": // 18 decimals → 6
		divisor := new(big.Int).Exp(big.NewInt(10), big.NewInt(12), nil)
		n.Div(n, divisor)
	case "USDC", "USDT": // already 6 decimals
		// no conversion needed
	case "BNB", "MATIC", "ARB", "OP": // 18 decimals → 6
		divisor := new(big.Int).Exp(big.NewInt(10), big.NewInt(12), nil)
		n.Div(n, divisor)
	default: // assume 18 decimals
		divisor := new(big.Int).Exp(big.NewInt(10), big.NewInt(12), nil)
		n.Div(n, divisor)
	}
	if n.Sign() == 0 {
		return "", fmt.Errorf("amount too small after conversion")
	}
	return n.String(), nil
}

// ProcessInboundAttestation records an attestation and mints if threshold reached.
func (k Keeper) ProcessInboundAttestation(
	ctx context.Context,
	validator, chain, txHash string,
	logIndex uint64,
	sender, recipient, token, amount string,
) (req types.InboundRequest, minted bool, err error) {
	sdkCtx := sdk.UnwrapSDKContext(ctx)
	params := k.GetParams(ctx)
	id := RequestID(chain, txHash, logIndex)

	// Get or create the request
	req, rerr := k.GetInboundRequest(ctx, id)
	if rerr != nil {
		// New request
		amitisAmount, err := convertAmount(token, amount)
		if err != nil {
			return req, false, err
		}
		denom := types.DenomForChainToken(chain, token)
		req = types.InboundRequest{
			ID:             id,
			SourceChain:    chain,
			SourceTxHash:   txHash,
			SourceLogIndex: logIndex,
			Sender:         sender,
			Recipient:      recipient,
			Token:          token,
			Amount:         amount,
			AmitisAmount:   amitisAmount,
			AmitisDenom:    denom,
			Status:         types.StatusPending,
			CreatedHeight:  sdkCtx.BlockHeight(),
		}
		if err := k.SetInboundRequest(ctx, req); err != nil {
			return req, false, err
		}
	}

	if req.Status != types.StatusPending {
		return req, false, fmt.Errorf("request %s is already %s", id, req.Status)
	}

	// Record attestation
	count, err := k.AddAttestation(ctx, id, validator)
	if err != nil {
		return req, false, err
	}
	req.Attestations = count

	// Check threshold
	if count >= params.AttestationThreshold {
		// Mint wrapped asset to recipient
		recipientAddr, err := sdk.AccAddressFromBech32(req.Recipient)
		if err != nil {
			return req, false, fmt.Errorf("invalid recipient: %w", err)
		}
		mintAmt, ok := sdkmath.NewIntFromString(req.AmitisAmount)
		if !ok {
			return req, false, fmt.Errorf("invalid amitis amount: %s", req.AmitisAmount)
		}
		coin := sdk.NewCoin(req.AmitisDenom, mintAmt)
		if err := k.BankKeeper.MintCoins(sdkCtx, types.ModuleName, sdk.NewCoins(coin)); err != nil {
			return req, false, fmt.Errorf("mint failed: %w", err)
		}
		if err := k.BankKeeper.SendCoinsFromModuleToAccount(sdkCtx, types.ModuleName, recipientAddr, sdk.NewCoins(coin)); err != nil {
			return req, false, fmt.Errorf("send minted coins failed: %w", err)
		}
		req.Status = types.StatusComplete
		req.CompletedHeight = sdkCtx.BlockHeight()
		minted = true

		sdkCtx.EventManager().EmitEvent(sdk.NewEvent(
			"bridge_inbound_complete",
			sdk.NewAttribute("request_id", id),
			sdk.NewAttribute("recipient", req.Recipient),
			sdk.NewAttribute("denom", req.AmitisDenom),
			sdk.NewAttribute("amount", req.AmitisAmount),
			sdk.NewAttribute("source_chain", chain),
			sdk.NewAttribute("source_tx", txHash),
		))
	}

	if err := k.SetInboundRequest(ctx, req); err != nil {
		return req, false, err
	}
	return req, minted, nil
}

// ── OUTBOUND EXECUTION (burn + create withdrawal record) ─────────────────────

func (k Keeper) ProcessWithdrawal(
	ctx context.Context,
	sender, destChain, destAddress, denom, amount string,
) (types.OutboundRequest, error) {
	sdkCtx := sdk.UnwrapSDKContext(ctx)

	senderAddr, err := sdk.AccAddressFromBech32(sender)
	if err != nil {
		return types.OutboundRequest{}, fmt.Errorf("invalid sender: %w", err)
	}

	burnAmt, ok := sdkmath.NewIntFromString(amount)
	if !ok || burnAmt.IsZero() {
		return types.OutboundRequest{}, fmt.Errorf("invalid amount: %s", amount)
	}

	coin := sdk.NewCoin(denom, burnAmt)

	// Send from user to module, then burn
	if err := k.BankKeeper.SendCoinsFromAccountToModule(sdkCtx, senderAddr, types.ModuleName, sdk.NewCoins(coin)); err != nil {
		return types.OutboundRequest{}, fmt.Errorf("failed to collect coins for burn: %w", err)
	}
	if err := k.BankKeeper.BurnCoins(sdkCtx, types.ModuleName, sdk.NewCoins(coin)); err != nil {
		return types.OutboundRequest{}, fmt.Errorf("burn failed: %w", err)
	}

	// Derive EVM amount from amitis denom amount
	// Reverse of convertAmount: multiply back by 1e12 for 18-decimal tokens
	token := tokenFromDenom(denom)
	evmAmount := deriveEVMAmount(token, amount)

	id := RequestID(destChain, sender+destAddress, uint64(sdkCtx.BlockHeight()))
	req := types.OutboundRequest{
		ID:            id,
		DestChain:     destChain,
		DestAddress:   destAddress,
		Sender:        sender,
		AmitisDenom:   denom,
		AmitisAmount:  amount,
		Token:         token,
		Amount:        evmAmount,
		Status:        types.StatusPending,
		CreatedHeight: sdkCtx.BlockHeight(),
	}

	if err := k.SetOutboundRequest(ctx, req); err != nil {
		return req, err
	}

	sdkCtx.EventManager().EmitEvent(sdk.NewEvent(
		"bridge_outbound_initiated",
		sdk.NewAttribute("request_id", id),
		sdk.NewAttribute("sender", sender),
		sdk.NewAttribute("dest_chain", destChain),
		sdk.NewAttribute("dest_address", destAddress),
		sdk.NewAttribute("denom", denom),
		sdk.NewAttribute("amount", amount),
	))

	return req, nil
}

func tokenFromDenom(denom string) string {
	switch denom {
	case types.DenomWrappedETH:
		return "ETH"
	case types.DenomWrappedUSDC:
		return "USDC"
	case types.DenomWrappedBNB:
		return "BNB"
	case types.DenomWrappedMATIC:
		return "MATIC"
	case types.DenomWrappedARB:
		return "ARB"
	case types.DenomWrappedOP:
		return "OP"
	case types.DenomWrappedWBTC:
		return "WBTC"
	default:
		return denom
	}
}

func deriveEVMAmount(token, amitisAmount string) string {
	n := new(big.Int)
	n.SetString(amitisAmount, 10)
	switch token {
	case "ETH", "WBTC", "BNB", "MATIC", "ARB", "OP":
		multiplier := new(big.Int).Exp(big.NewInt(10), big.NewInt(12), nil)
		n.Mul(n, multiplier)
	}
	return n.String()
}

// ── EXPIRY (called from EndBlock) ─────────────────────────────────────────────

func (k Keeper) ExpireStaleRequests(ctx context.Context) {
	sdkCtx := sdk.UnwrapSDKContext(ctx)
	params := k.GetParams(ctx)
	currentHeight := sdkCtx.BlockHeight()

	reqs, err := k.GetAllInboundRequests(ctx)
	if err != nil {
		return
	}
	for _, req := range reqs {
		if req.Status == types.StatusPending &&
			currentHeight-req.CreatedHeight > params.RequestExpiry {
			req.Status = types.StatusExpired
			_ = k.SetInboundRequest(ctx, req)
		}
	}
}

// ── STATS ─────────────────────────────────────────────────────────────────────

type BridgeStats struct {
	TotalInbound   int `json:"total_inbound"`
	PendingInbound int `json:"pending_inbound"`
	CompleteInbound int `json:"complete_inbound"`
	TotalOutbound  int `json:"total_outbound"`
	PendingOutbound int `json:"pending_outbound"`
}

func (k Keeper) GetStats(ctx context.Context) (BridgeStats, error) {
	inbound, err := k.GetAllInboundRequests(ctx)
	if err != nil {
		return BridgeStats{}, err
	}
	outbound, err := k.GetAllOutboundRequests(ctx)
	if err != nil {
		return BridgeStats{}, err
	}
	stats := BridgeStats{
		TotalInbound:  len(inbound),
		TotalOutbound: len(outbound),
	}
	for _, r := range inbound {
		switch r.Status {
		case types.StatusPending:
			stats.PendingInbound++
		case types.StatusComplete:
			stats.CompleteInbound++
		}
	}
	for _, r := range outbound {
		if r.Status == types.StatusPending {
			stats.PendingOutbound++
		}
	}
	return stats, nil
}
