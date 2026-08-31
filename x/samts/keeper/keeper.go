package keeper

import (
	"context"
	"encoding/json"
	"fmt"
	"math/big"

	"cosmossdk.io/core/store"
	sdkmath "cosmossdk.io/math"
	sdk "github.com/cosmos/cosmos-sdk/types"
	bankkeeper "github.com/cosmos/cosmos-sdk/x/bank/keeper"
	authtypes "github.com/cosmos/cosmos-sdk/x/auth/types"

	"github.com/amitis-network/amitis-network/x/samts/types"
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

// --- Position management ---

func (k Keeper) SetPosition(ctx context.Context, pos types.ValidatorPosition) error {
	store := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyPosition, []byte(pos.ValidatorAddress)...)
	bz, err := json.Marshal(pos)
	if err != nil {
		return err
	}
	return store.Set(key, bz)
}

func (k Keeper) GetPosition(ctx context.Context, valAddr string) (types.ValidatorPosition, error) {
	store := k.storeService.OpenKVStore(ctx)
	key := append(types.KeyPosition, []byte(valAddr)...)
	bz, err := store.Get(key)
	if err != nil || bz == nil {
		return types.ValidatorPosition{}, fmt.Errorf("position not found for %s", valAddr)
	}
	var pos types.ValidatorPosition
	return pos, json.Unmarshal(bz, &pos)
}

func (k Keeper) GetAllPositions(ctx context.Context) ([]types.ValidatorPosition, error) {
	store := k.storeService.OpenKVStore(ctx)
	iter, err := store.Iterator(types.KeyPosition, append(types.KeyPosition, 0xFF))
	if err != nil {
		return nil, err
	}
	defer iter.Close()
	var positions []types.ValidatorPosition
	for ; iter.Valid(); iter.Next() {
		var pos types.ValidatorPosition
		if err := json.Unmarshal(iter.Value(), &pos); err != nil {
			return nil, err
		}
		positions = append(positions, pos)
	}
	return positions, nil
}

// --- Params ---

func (k Keeper) GetParams(ctx context.Context) types.Params {
	store := k.storeService.OpenKVStore(ctx)
	bz, err := store.Get(types.KeyParams)
	if err != nil || bz == nil {
		return types.Params{
			FlashLoanFeeRate: types.DefaultFlashLoanFeeRate,
			MaxLoanFraction:  types.DefaultMaxLoanFraction,
		}
	}
	var params types.Params
	_ = json.Unmarshal(bz, &params)
	return params
}

func (k Keeper) SetParams(ctx context.Context, params types.Params) error {
	store := k.storeService.OpenKVStore(ctx)
	bz, err := json.Marshal(params)
	if err != nil {
		return err
	}
	return store.Set(types.KeyParams, bz)
}

// --- Pool calculations ---

// TotalPool returns total sAMTS across all validators
func (k Keeper) TotalPool(ctx context.Context) (*big.Int, error) {
	positions, err := k.GetAllPositions(ctx)
	if err != nil {
		return nil, err
	}
	total := new(big.Int)
	for _, p := range positions {
		n := new(big.Int)
		n.SetString(p.SamtsBalance, 10)
		total.Add(total, n)
	}
	return total, nil
}

// MaxLoan returns the maximum flash loan amount (50% of pool)
func (k Keeper) MaxLoan(ctx context.Context) (*big.Int, error) {
	total, err := k.TotalPool(ctx)
	if err != nil {
		return nil, err
	}
	params := k.GetParams(ctx)
	// Parse max fraction
	fracStr := params.MaxLoanFraction
	if fracStr == "" {
		fracStr = "0.5"
	}
	// Multiply total by fraction: total * 5 / 10 for 0.5
	fracNum, fracDen := parseFraction(fracStr)
	max := new(big.Int).Mul(total, big.NewInt(int64(fracNum)))
	max.Div(max, big.NewInt(int64(fracDen)))
	return max, nil
}

// --- Flash loan ---

func (k Keeper) ExecuteFlashLoan(ctx context.Context, borrower string, amount *big.Int) (fee *big.Int, err error) {
	sdkCtx := sdk.UnwrapSDKContext(ctx)

	// Check pool size
	maxLoan, err := k.MaxLoan(ctx)
	if err != nil {
		return nil, err
	}
	if amount.Cmp(maxLoan) > 0 {
		return nil, fmt.Errorf("amount %s exceeds max loan %s", amount, maxLoan)
	}

	// Calculate fee: amount * 0.05% = amount * 5 / 10000
	params := k.GetParams(ctx)
	_ = params
	fee = new(big.Int).Mul(amount, big.NewInt(5))
	fee.Div(fee, big.NewInt(10000))

	// Send AMTS from module to borrower
	borrowerAddr, err := sdk.AccAddressFromBech32(borrower)
	if err != nil {
		return nil, err
	}
	amtInt, _ := sdkmath.NewIntFromString(amount.String())
	loanCoin := sdk.NewCoin("uamts", amtInt)
	if err := k.BankKeeper.SendCoinsFromModuleToAccount(sdkCtx, types.ModuleName, borrowerAddr, sdk.NewCoins(loanCoin)); err != nil {
		return nil, fmt.Errorf("flash loan disbursement: %w", err)
	}

	// Borrower must repay principal + fee in same TX via ante handler
	// Fee goes to fee collector for distribution to validators
	feeInt, _ := sdkmath.NewIntFromString(fee.String())
	repayTotal := amtInt.Add(feeInt)
	repayCoin := sdk.NewCoin("uamts", repayTotal)
	if err := k.BankKeeper.SendCoinsFromAccountToModule(sdkCtx, borrowerAddr, types.ModuleName, sdk.NewCoins(repayCoin)); err != nil {
		return nil, fmt.Errorf("flash loan repayment failed — loan reverted: %w", err)
	}

	// Route fee to fee collector
	feeCoin := sdk.NewCoin("uamts", feeInt)
	if err := k.BankKeeper.SendCoinsFromModuleToModule(sdkCtx, types.ModuleName, k.FeeCollector, sdk.NewCoins(feeCoin)); err != nil {
		return nil, fmt.Errorf("fee routing: %w", err)
	}

	return fee, nil
}

// parseFraction converts "0.5" -> (5, 10), "0.05" -> (5, 100), etc.
func parseFraction(s string) (num, den int) {
	switch s {
	case "0.5":
		return 5, 10
	case "0.25":
		return 25, 100
	default:
		return 5, 10
	}
}
