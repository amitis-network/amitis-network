package keeper

import (
	"context"

	"github.com/amitis-network/amitis-network/x/amitisbridge/types"
)

type MsgServer struct {
	Keeper
}

func NewMsgServer(k Keeper) MsgServer {
	return MsgServer{Keeper: k}
}

// SubmitAttestation is called by validators when they observe a lock event
// on a source EVM chain. Once the attestation threshold is reached,
// the module mints the wrapped asset to the recipient on Amitis.
func (m MsgServer) SubmitAttestation(ctx context.Context, msg *types.MsgSubmitAttestation) (*types.MsgSubmitAttestationResponse, error) {
	if err := msg.ValidateBasic(); err != nil {
		return nil, err
	}

	req, minted, err := m.Keeper.ProcessInboundAttestation(
		ctx,
		msg.Validator,
		msg.SourceChain,
		msg.SourceTxHash,
		msg.SourceLogIndex,
		msg.Sender,
		msg.Recipient,
		msg.Token,
		msg.Amount,
	)
	if err != nil {
		return nil, err
	}

	status := "attested"
	if minted {
		status = "completed"
	}

	return &types.MsgSubmitAttestationResponse{
		RequestID: req.ID,
		Status:    status,
		Minted:    minted,
	}, nil
}

// InitiateWithdrawal burns wrapped assets on Amitis and creates a pending
// outbound request. Validators monitor outbound requests and submit the
// corresponding release transaction on the destination EVM chain.
func (m MsgServer) InitiateWithdrawal(ctx context.Context, msg *types.MsgInitiateWithdrawal) (*types.MsgInitiateWithdrawalResponse, error) {
	if err := msg.ValidateBasic(); err != nil {
		return nil, err
	}

	req, err := m.Keeper.ProcessWithdrawal(
		ctx,
		msg.Sender,
		msg.DestChain,
		msg.DestAddress,
		msg.Denom,
		msg.Amount,
	)
	if err != nil {
		return nil, err
	}

	return &types.MsgInitiateWithdrawalResponse{
		RequestID: req.ID,
		Status:    req.Status,
	}, nil
}

// UpdateBridgeParams updates bridge parameters via governance.
func (m MsgServer) UpdateBridgeParams(ctx context.Context, msg *types.MsgUpdateBridgeParams) (*types.MsgUpdateBridgeParamsResponse, error) {
	if err := msg.ValidateBasic(); err != nil {
		return nil, err
	}
	if err := m.Keeper.SetParams(ctx, msg.Params); err != nil {
		return nil, err
	}
	return &types.MsgUpdateBridgeParamsResponse{}, nil
}
