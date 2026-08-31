package keeper

import (
	"context"
	"math/big"

	"github.com/amitis-network/amitis-network/x/samts/types"
)

type MsgServer struct {
	Keeper
	types.UnimplementedMsgServer
}

var _ types.MsgServer = MsgServer{}

func NewMsgServer(k Keeper) MsgServer {
	return MsgServer{Keeper: k}
}

func (m MsgServer) FlashLoan(ctx context.Context, msg *types.MsgFlashLoan) (*types.MsgFlashLoanResponse, error) {
	amount := new(big.Int)
	amount.SetString(msg.Amount, 10)

	fee, err := m.Keeper.ExecuteFlashLoan(ctx, msg.Borrower, amount)
	if err != nil {
		return nil, err
	}

	return &types.MsgFlashLoanResponse{
		AmountBorrowed: msg.Amount,
		Fee:            fee.String(),
	}, nil
}
