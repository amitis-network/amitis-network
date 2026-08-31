package keeper

import (
	"context"
	"encoding/json"
	"fmt"

	"cosmossdk.io/core/store"
	sdk "github.com/cosmos/cosmos-sdk/types"
	authkeeper "github.com/cosmos/cosmos-sdk/x/auth/keeper"
	bankkeeper "github.com/cosmos/cosmos-sdk/x/bank/keeper"

	"github.com/amitis-network/amitis-network/x/cooperative/types"
)

type Keeper struct {
	storeService     store.KVStoreService
	bankKeeper       bankkeeper.Keeper
	accountKeeper    authkeeper.AccountKeeper
	feeCollectorName string
}

func NewKeeper(
	storeService store.KVStoreService,
	bankKeeper bankkeeper.Keeper,
	accountKeeper authkeeper.AccountKeeper,
	feeCollectorName string,
) Keeper {
	return Keeper{
		storeService:     storeService,
		bankKeeper:       bankKeeper,
		accountKeeper:    accountKeeper,
		feeCollectorName: feeCollectorName,
	}
}

// ProcessFeeRebate sends 80% of fees back to the tx sender
func (k Keeper) ProcessFeeRebate(ctx context.Context, sender sdk.AccAddress, fees sdk.Coins) error {
	if sender == nil || fees.IsZero() {
		return nil
	}

	sdkCtx := sdk.UnwrapSDKContext(ctx)

	// Calculate 80% rebate
	rebateCoins := sdk.NewCoins()
	for _, fee := range fees {
		rebateAmount := fee.Amount.MulRaw(int64(types.RebateNumerator)).QuoRaw(int64(types.RebateDenominator))
		if rebateAmount.IsPositive() {
			rebateCoins = rebateCoins.Add(sdk.NewCoin(fee.Denom, rebateAmount))
		}
	}

	if rebateCoins.IsZero() {
		return nil
	}

	// Get fee collector address via AccountKeeper
	feeCollectorAddr := k.accountKeeper.GetModuleAddress(k.feeCollectorName)
	if feeCollectorAddr == nil {
		return fmt.Errorf("fee collector module account not found")
	}

	// Check fee collector has sufficient balance before sending
	for _, coin := range rebateCoins {
		balance := k.bankKeeper.GetBalance(sdkCtx, feeCollectorAddr, coin.Denom)
		if balance.IsLT(coin) {
			// Not enough in fee collector — skip rebate rather than fail tx
			return nil
		}
	}

	if err := k.bankKeeper.SendCoinsFromModuleToAccount(sdkCtx, k.feeCollectorName, sender, rebateCoins); err != nil {
		return fmt.Errorf("failed to send fee rebate: %w", err)
	}

	// Emit rebate event
	sdkCtx.EventManager().EmitEvent(
		sdk.NewEvent(
			types.EventTypeRebate,
			sdk.NewAttribute(types.AttributeKeyRebateAddress, sender.String()),
			sdk.NewAttribute(types.AttributeKeyRebateAmount, rebateCoins.String()),
		),
	)

	return nil
}

// GetTotalRebatesPaid returns the lifetime total of rebates paid
func (k Keeper) GetTotalRebatesPaid(ctx context.Context) (string, error) {
	kvStore := k.storeService.OpenKVStore(ctx)
	bz, err := kvStore.Get([]byte("total_rebates"))
	if err != nil || bz == nil {
		return "0", nil
	}
	var total string
	if err := json.Unmarshal(bz, &total); err != nil {
		return "0", nil
	}
	return total, nil
}

// SetTotalRebatesPaid stores the lifetime total
func (k Keeper) SetTotalRebatesPaid(ctx context.Context, total string) error {
	kvStore := k.storeService.OpenKVStore(ctx)
	bz, err := json.Marshal(total)
	if err != nil {
		return err
	}
	return kvStore.Set([]byte("total_rebates"), bz)
}
