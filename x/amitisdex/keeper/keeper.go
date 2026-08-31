package keeper

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"math/big"

	"cosmossdk.io/collections"
	"cosmossdk.io/core/store"
	sdkmath "cosmossdk.io/math"
	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	bankkeeper "github.com/cosmos/cosmos-sdk/x/bank/keeper"

	"github.com/amitis-network/amitis-network/x/amitisdex/types"
)

type Keeper struct {
	cdc          codec.BinaryCodec
	storeService store.KVStoreService
	BankKeeper   bankkeeper.Keeper
	FeeCollector string // module account name for fee distribution

	Pools      collections.Map[uint64, []byte]
	Positions  collections.Map[string, []byte]
	NextPoolId collections.Item[uint64]
}

func NewKeeper(
	cdc codec.BinaryCodec,
	storeService store.KVStoreService,
	bankKeeper bankkeeper.Keeper,
) Keeper {
	sb := collections.NewSchemaBuilder(storeService)
	return Keeper{
		cdc:          cdc,
		storeService: storeService,
		BankKeeper:   bankKeeper,
		FeeCollector: "fee_collector",
		Pools:        collections.NewMap(sb, collections.NewPrefix(1), "pools", collections.Uint64Key, collections.BytesValue),
		Positions:    collections.NewMap(sb, collections.NewPrefix(2), "positions", collections.StringKey, collections.BytesValue),
		NextPoolId:   collections.NewItem(sb, collections.NewPrefix(3), "next_pool_id", collections.Uint64Value),
	}
}

func (k Keeper) GetPool(ctx context.Context, id uint64) (*types.Pool, error) {
	bz, err := k.Pools.Get(ctx, id)
	if err != nil {
		return nil, fmt.Errorf("pool %d not found: %w", id, err)
	}
	var p types.Pool
	if err := json.Unmarshal(bz, &p); err != nil {
		return nil, fmt.Errorf("unmarshal pool %d: %w", id, err)
	}
	return &p, nil
}

func (k Keeper) SetPool(ctx context.Context, p *types.Pool) error {
	bz, err := json.Marshal(p)
	if err != nil {
		return fmt.Errorf("marshal pool: %w", err)
	}
	return k.Pools.Set(ctx, p.Id, bz)
}

func (k Keeper) GetAllPools(ctx context.Context) ([]*types.Pool, error) {
	var pools []*types.Pool
	err := k.Pools.Walk(ctx, nil, func(_ uint64, bz []byte) (bool, error) {
		var p types.Pool
		if err := json.Unmarshal(bz, &p); err != nil {
			return true, err
		}
		pools = append(pools, &p)
		return false, nil
	})
	return pools, err
}

func (k Keeper) GetPoolByPair(ctx context.Context, denomA, denomB string) (*types.Pool, error) {
	a, b := types.SortDenoms(denomA, denomB)
	pools, err := k.GetAllPools(ctx)
	if err != nil {
		return nil, err
	}
	for _, p := range pools {
		if p.DenomA == a && p.DenomB == b {
			return p, nil
		}
	}
	return nil, fmt.Errorf("no pool found for %s/%s", a, b)
}

func (k Keeper) getNextPoolId(ctx context.Context) (uint64, error) {
	id, err := k.NextPoolId.Get(ctx)
	if err != nil {
		return 1, nil
	}
	return id, nil
}

func (k Keeper) setNextPoolId(ctx context.Context, id uint64) error {
	return k.NextPoolId.Set(ctx, id)
}

func posKey(poolId uint64, addr string) string {
	return fmt.Sprintf("%d:%s", poolId, addr)
}

func (k Keeper) GetPosition(ctx context.Context, poolId uint64, addr string) (*types.LiquidityPosition, error) {
	bz, err := k.Positions.Get(ctx, posKey(poolId, addr))
	if err != nil {
		return &types.LiquidityPosition{PoolId: poolId, Address: addr, Shares: "0"}, nil
	}
	var pos types.LiquidityPosition
	if err := json.Unmarshal(bz, &pos); err != nil {
		return nil, fmt.Errorf("unmarshal position: %w", err)
	}
	return &pos, nil
}

func (k Keeper) SetPosition(ctx context.Context, pos *types.LiquidityPosition) error {
	bz, err := json.Marshal(pos)
	if err != nil {
		return err
	}
	return k.Positions.Set(ctx, posKey(pos.PoolId, pos.Address), bz)
}

func (k Keeper) GetAllPositions(ctx context.Context) ([]*types.LiquidityPosition, error) {
	var positions []*types.LiquidityPosition
	err := k.Positions.Walk(ctx, nil, func(_ string, bz []byte) (bool, error) {
		var pos types.LiquidityPosition
		if err := json.Unmarshal(bz, &pos); err != nil {
			return true, err
		}
		positions = append(positions, &pos)
		return false, nil
	})
	return positions, err
}

func (k Keeper) GetPositionsByAddress(ctx context.Context, addr string) ([]*types.LiquidityPosition, error) {
	all, err := k.GetAllPositions(ctx)
	if err != nil {
		return nil, err
	}
	var result []*types.LiquidityPosition
	for _, p := range all {
		if p.Address == addr {
			result = append(result, p)
		}
	}
	return result, nil
}

func (k Keeper) CreatePool(ctx context.Context, msg *types.MsgCreatePool) (*types.Pool, error) {
	if err := types.ValidateCreatePool(msg); err != nil {
		return nil, err
	}

	dA, dB := types.SortDenoms(msg.DenomA, msg.DenomB)
	var amtA, amtB string
	if dA == msg.DenomA {
		amtA, amtB = msg.AmountA, msg.AmountB
	} else {
		amtA, amtB = msg.AmountB, msg.AmountA
	}

	if _, err := k.GetPoolByPair(ctx, dA, dB); err == nil {
		return nil, fmt.Errorf("pool already exists for %s/%s", dA, dB)
	}

	_, _, shares, err := types.ComputeAddLiquidity("0", "0", "0", amtA, amtB)
	if err != nil {
		return nil, fmt.Errorf("compute initial liquidity: %w", err)
	}

	poolId, err := k.getNextPoolId(ctx)
	if err != nil {
		return nil, err
	}
	if err := k.setNextPoolId(ctx, poolId+1); err != nil {
		return nil, err
	}

	sdkCtx := sdk.UnwrapSDKContext(ctx)
	creatorAddr, err := sdk.AccAddressFromBech32(msg.Creator)
	if err != nil {
		return nil, fmt.Errorf("invalid creator address: %w", err)
	}

	coinsIn, err := parseCoins(amtA, dA, amtB, dB)
	if err != nil {
		return nil, err
	}

	if err := k.BankKeeper.SendCoinsFromAccountToModule(sdkCtx, creatorAddr, types.ModuleName, coinsIn); err != nil {
		return nil, fmt.Errorf("failed to transfer tokens to pool: %w", err)
	}

	pool := &types.Pool{
		Id:          poolId,
		DenomA:      dA,
		DenomB:      dB,
		ReserveA:    amtA,
		ReserveB:    amtB,
		TotalShares: shares,
	}
	if err := k.SetPool(ctx, pool); err != nil {
		return nil, err
	}

	pos := &types.LiquidityPosition{PoolId: poolId, Address: msg.Creator, Shares: shares}
	if err := k.SetPosition(ctx, pos); err != nil {
		return nil, err
	}

	return pool, nil
}

func (k Keeper) AddLiquidity(ctx context.Context, msg *types.MsgAddLiquidity) (adjA, adjB, shares string, err error) {
	pool, err := k.GetPool(ctx, msg.PoolId)
	if err != nil {
		return "", "", "", err
	}

	adjA, adjB, shares, err = types.ComputeAddLiquidity(pool.ReserveA, pool.ReserveB, pool.TotalShares, msg.AmountA, msg.AmountB)
	if err != nil {
		return "", "", "", err
	}

	sdkCtx := sdk.UnwrapSDKContext(ctx)
	senderAddr, err := sdk.AccAddressFromBech32(msg.Sender)
	if err != nil {
		return "", "", "", err
	}

	coinsIn, err := parseCoins(adjA, pool.DenomA, adjB, pool.DenomB)
	if err != nil {
		return "", "", "", err
	}

	if err := k.BankKeeper.SendCoinsFromAccountToModule(sdkCtx, senderAddr, types.ModuleName, coinsIn); err != nil {
		return "", "", "", fmt.Errorf("failed to transfer tokens to pool: %w", err)
	}

	pool.ReserveA = addStr(pool.ReserveA, adjA)
	pool.ReserveB = addStr(pool.ReserveB, adjB)
	pool.TotalShares = addStr(pool.TotalShares, shares)
	if err := k.SetPool(ctx, pool); err != nil {
		return "", "", "", err
	}

	pos, _ := k.GetPosition(ctx, msg.PoolId, msg.Sender)
	pos.Shares = addStr(pos.Shares, shares)
	if err := k.SetPosition(ctx, pos); err != nil {
		return "", "", "", err
	}

	return adjA, adjB, shares, nil
}

func (k Keeper) RemoveLiquidity(ctx context.Context, msg *types.MsgRemoveLiquidity) (amountA, amountB string, err error) {
	pool, err := k.GetPool(ctx, msg.PoolId)
	if err != nil {
		return "", "", err
	}

	pos, err := k.GetPosition(ctx, msg.PoolId, msg.Sender)
	if err != nil {
		return "", "", err
	}

	burn := mustBigInt(msg.SharesToBurn)
	held := mustBigInt(pos.Shares)
	if burn.Cmp(held) > 0 {
		return "", "", errors.New("insufficient LP shares")
	}

	amountA, amountB, err = types.ComputeRemoveLiquidity(pool.ReserveA, pool.ReserveB, pool.TotalShares, msg.SharesToBurn)
	if err != nil {
		return "", "", err
	}

	sdkCtx := sdk.UnwrapSDKContext(ctx)
	senderAddr, err := sdk.AccAddressFromBech32(msg.Sender)
	if err != nil {
		return "", "", err
	}

	coinsOut, err := parseCoins(amountA, pool.DenomA, amountB, pool.DenomB)
	if err != nil {
		return "", "", err
	}

	if err := k.BankKeeper.SendCoinsFromModuleToAccount(sdkCtx, types.ModuleName, senderAddr, coinsOut); err != nil {
		return "", "", fmt.Errorf("failed to send tokens from pool: %w", err)
	}

	pool.ReserveA = subStr(pool.ReserveA, amountA)
	pool.ReserveB = subStr(pool.ReserveB, amountB)
	pool.TotalShares = subStr(pool.TotalShares, msg.SharesToBurn)
	if err := k.SetPool(ctx, pool); err != nil {
		return "", "", err
	}

	pos.Shares = subStr(pos.Shares, msg.SharesToBurn)
	if err := k.SetPosition(ctx, pos); err != nil {
		return "", "", err
	}

	return amountA, amountB, nil
}

func (k Keeper) SwapExactIn(ctx context.Context, msg *types.MsgSwapExactIn) (amountOut string, err error) {
	pool, err := k.GetPool(ctx, msg.PoolId)
	if err != nil {
		return "", err
	}

	var reserveIn, reserveOut, denomOut string
	if msg.DenomIn == pool.DenomA {
		reserveIn, reserveOut, denomOut = pool.ReserveA, pool.ReserveB, pool.DenomB
	} else if msg.DenomIn == pool.DenomB {
		reserveIn, reserveOut, denomOut = pool.ReserveB, pool.ReserveA, pool.DenomA
	} else {
		return "", fmt.Errorf("denom %s not in pool %d", msg.DenomIn, msg.PoolId)
	}

	// Fee split:
	// 0.25% stays in pool (LP fee) — computed via 997/1000 factor on amountIn
	// 0.05% goes to fee collector (validator fee) — taken from amountIn before swap
	aIn := mustBigInt(msg.AmountIn)

	// Validator fee: 0.05% = 5/10000 of amountIn
	validatorFee := new(big.Int).Mul(aIn, big.NewInt(5))
	validatorFee.Div(validatorFee, big.NewInt(10000))

	// Amount that enters the pool (after validator fee)
	amountInForPool := new(big.Int).Sub(aIn, validatorFee)

	amountOut, newRIn, newROut, err := types.ComputeSwapOutput(reserveIn, reserveOut, amountInForPool.String())
	if err != nil {
		return "", err
	}

	if msg.MinAmountOut != "" && msg.MinAmountOut != "0" {
		if mustBigInt(amountOut).Cmp(mustBigInt(msg.MinAmountOut)) < 0 {
			return "", fmt.Errorf("slippage: got %s %s, min required %s", amountOut, denomOut, msg.MinAmountOut)
		}
	}

	sdkCtx := sdk.UnwrapSDKContext(ctx)
	senderAddr, err := sdk.AccAddressFromBech32(msg.Sender)
	if err != nil {
		return "", err
	}

	// Transfer full amountIn from sender to module
	aInInt, ok := sdkmath.NewIntFromString(msg.AmountIn)
	if !ok {
		return "", fmt.Errorf("invalid amountIn: %s", msg.AmountIn)
	}
	coinIn := sdk.NewCoin(msg.DenomIn, aInInt)
	if err := k.BankKeeper.SendCoinsFromAccountToModule(sdkCtx, senderAddr, types.ModuleName, sdk.NewCoins(coinIn)); err != nil {
		return "", fmt.Errorf("transfer in: %w", err)
	}

	// Route validator fee (0.05%) to fee collector
	if validatorFee.Sign() > 0 {
		vFeeInt, _ := sdkmath.NewIntFromString(validatorFee.String())
		vFeeCoin := sdk.NewCoin(msg.DenomIn, vFeeInt)
		if err := k.BankKeeper.SendCoinsFromModuleToModule(sdkCtx, types.ModuleName, k.FeeCollector, sdk.NewCoins(vFeeCoin)); err != nil {
			return "", fmt.Errorf("validator fee routing: %w", err)
		}
	}

	// Send output tokens to sender
	aOut, ok2 := sdkmath.NewIntFromString(amountOut)
	if !ok2 {
		return "", fmt.Errorf("invalid amountOut: %s", amountOut)
	}
	coinOut := sdk.NewCoin(denomOut, aOut)
	if err := k.BankKeeper.SendCoinsFromModuleToAccount(sdkCtx, types.ModuleName, senderAddr, sdk.NewCoins(coinOut)); err != nil {
		return "", fmt.Errorf("transfer out: %w", err)
	}

	// Update pool reserves (LP fee stays in pool via amountInForPool)
	if msg.DenomIn == pool.DenomA {
		pool.ReserveA = newRIn
		pool.ReserveB = newROut
	} else {
		pool.ReserveB = newRIn
		pool.ReserveA = newROut
	}
	if err := k.SetPool(ctx, pool); err != nil {
		return "", err
	}

	return amountOut, nil
}

// -------- Helpers --------

func mustBigInt(s string) *big.Int {
	n := new(big.Int)
	n.SetString(s, 10)
	return n
}

func addStr(a, b string) string {
	x := mustBigInt(a)
	y := mustBigInt(b)
	return x.Add(x, y).String()
}

func subStr(a, b string) string {
	x := mustBigInt(a)
	y := mustBigInt(b)
	return x.Sub(x, y).String()
}

func parseCoins(amtA, denomA, amtB, denomB string) (sdk.Coins, error) {
	a, ok := sdkmath.NewIntFromString(amtA)
	if !ok {
		return nil, fmt.Errorf("invalid amount: %s", amtA)
	}
	b, ok2 := sdkmath.NewIntFromString(amtB)
	if !ok2 {
		return nil, fmt.Errorf("invalid amount: %s", amtB)
	}
	coins := sdk.NewCoins(
		sdk.NewCoin(denomA, a),
		sdk.NewCoin(denomB, b),
	)
	return coins, nil
}
