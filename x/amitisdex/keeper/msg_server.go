package keeper

import (
	"context"

	types "github.com/amitis-network/amitis-network/x/amitisdex/types"
)

// MsgServer implements the amitisdex MsgServer interface.
type MsgServer struct {
	Keeper
	types.UnimplementedMsgServer
}

var _ types.MsgServer = MsgServer{}

func NewMsgServer(k Keeper) MsgServer {
	return MsgServer{Keeper: k}
}

func (m MsgServer) CreatePool(ctx context.Context, msg *types.MsgCreatePool) (*types.MsgCreatePoolResponse, error) {
	pool, err := m.Keeper.CreatePool(ctx, msg)
	if err != nil {
		return nil, err
	}
	return &types.MsgCreatePoolResponse{PoolId: pool.Id}, nil
}

func (m MsgServer) AddLiquidity(ctx context.Context, msg *types.MsgAddLiquidity) (*types.MsgAddLiquidityResponse, error) {
	adjA, adjB, shares, err := m.Keeper.AddLiquidity(ctx, msg)
	if err != nil {
		return nil, err
	}
	return &types.MsgAddLiquidityResponse{AmountA: adjA, AmountB: adjB, Shares: shares}, nil
}

func (m MsgServer) RemoveLiquidity(ctx context.Context, msg *types.MsgRemoveLiquidity) (*types.MsgRemoveLiquidityResponse, error) {
	amtA, amtB, err := m.Keeper.RemoveLiquidity(ctx, msg)
	if err != nil {
		return nil, err
	}
	return &types.MsgRemoveLiquidityResponse{AmountA: amtA, AmountB: amtB}, nil
}

func (m MsgServer) SwapExactIn(ctx context.Context, msg *types.MsgSwapExactIn) (*types.MsgSwapExactInResponse, error) {
	amountOut, err := m.Keeper.SwapExactIn(ctx, msg)
	if err != nil {
		return nil, err
	}
	return &types.MsgSwapExactInResponse{AmountOut: amountOut}, nil
}
