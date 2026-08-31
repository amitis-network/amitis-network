package simapp

import (
	"errors"

	errorsmod "cosmossdk.io/errors"
	sdkmath "cosmossdk.io/math"
	circuitante "cosmossdk.io/x/circuit/ante"

	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
	"github.com/cosmos/cosmos-sdk/x/auth/ante"
	banktypes "github.com/cosmos/cosmos-sdk/x/bank/types"

	cooperativekeeper "github.com/amitis-network/amitis-network/x/cooperative/keeper"
)

// HandlerOptions are the options required for constructing a default SDK AnteHandler.
type HandlerOptions struct {
	ante.HandlerOptions
	CircuitKeeper     circuitante.CircuitBreaker
	CooperativeKeeper cooperativekeeper.Keeper
}

// -----------------------------------------------------------------------
// MonetaryFeeDecorator
// -----------------------------------------------------------------------
// For transactions that transfer value (MsgSend, MsgMultiSend), this
// decorator requires a fee of 0.3% of the transferred amount instead of
// the normal gas-based fee. The fee replaces (not stacks on) gas fees.
// Non-monetary transactions fall through to normal gas fee logic.

type MonetaryFeeDecorator struct{}

func NewMonetaryFeeDecorator() MonetaryFeeDecorator {
	return MonetaryFeeDecorator{}
}

// monetaryAmount inspects tx messages and returns the total uamts value
// being transferred, and whether any monetary messages were found.
func monetaryAmount(msgs []sdk.Msg) (sdkmath.Int, bool) {
	total := sdkmath.ZeroInt()
	for _, msg := range msgs {
		switch m := msg.(type) {
		case *banktypes.MsgSend:
			for _, coin := range m.Amount {
				if coin.Denom == "uamts" {
					total = total.Add(coin.Amount)
				}
			}
		case *banktypes.MsgMultiSend:
			for _, input := range m.Inputs {
				for _, coin := range input.Coins {
					if coin.Denom == "uamts" {
						total = total.Add(coin.Amount)
					}
				}
			}
		}
	}
	return total, total.IsPositive()
}

func (mfd MonetaryFeeDecorator) AnteHandle(ctx sdk.Context, tx sdk.Tx, simulate bool, next sdk.AnteHandler) (sdk.Context, error) {
	// Skip on simulation
	if simulate {
		return next(ctx, tx, simulate)
	}

	feeTx, ok := tx.(sdk.FeeTx)
	if !ok {
		return next(ctx, tx, simulate)
	}

	amount, isMonetary := monetaryAmount(tx.GetMsgs())
	if !isMonetary {
		// Not a monetary tx — let normal gas fee logic handle it
		return next(ctx, tx, simulate)
	}

	// 0.3% = amount * 3 / 1000
	requiredFee := amount.Mul(sdkmath.NewInt(3)).Quo(sdkmath.NewInt(1000))

	// Floor: at least 1 uamts
	if requiredFee.IsZero() {
		requiredFee = sdkmath.OneInt()
	}

	provided := feeTx.GetFee().AmountOf("uamts")
	if provided.LT(requiredFee) {
		return ctx, errorsmod.Wrapf(
			sdkerrors.ErrInsufficientFee,
			"monetary transaction requires 0.3%% fee: need %suamts, got %suamts",
			requiredFee.String(), provided.String(),
		)
	}

	return next(ctx, tx, simulate)
}

// -----------------------------------------------------------------------
// FeeRebateDecorator
// -----------------------------------------------------------------------
// Sends 80% of tx fees back to the fee payer after fees are deducted.

type FeeRebateDecorator struct {
	cooperativeKeeper cooperativekeeper.Keeper
}

func NewFeeRebateDecorator(ck cooperativekeeper.Keeper) FeeRebateDecorator {
	return FeeRebateDecorator{cooperativeKeeper: ck}
}

func (frd FeeRebateDecorator) AnteHandle(ctx sdk.Context, tx sdk.Tx, simulate bool, next sdk.AnteHandler) (sdk.Context, error) {
	// Process next decorators first (fees must be deducted before rebate)
	ctx, err := next(ctx, tx, simulate)
	if err != nil {
		return ctx, err
	}

	// Skip rebate on simulation or CheckTx
	if simulate || ctx.IsCheckTx() {
		return ctx, nil
	}

	feeTx, ok := tx.(sdk.FeeTx)
	if !ok {
		return ctx, nil
	}

	fees := feeTx.GetFee()
	if fees.IsZero() {
		return ctx, nil
	}

	feePayer := sdk.AccAddress(feeTx.FeePayer())
	if feePayer == nil {
		return ctx, nil
	}

	// Process 80% rebate — never fail the tx on rebate error
	if err := frd.cooperativeKeeper.ProcessFeeRebate(ctx, feePayer, fees); err != nil {
		ctx.Logger().Error("fee rebate failed", "error", err, "payer", feePayer.String())
	}

	return ctx, nil
}

// -----------------------------------------------------------------------
// NewAnteHandler
// -----------------------------------------------------------------------

func NewAnteHandler(options HandlerOptions) (sdk.AnteHandler, error) {
	if options.AccountKeeper == nil {
		return nil, errors.New("account keeper is required for ante builder")
	}
	if options.BankKeeper == nil {
		return nil, errors.New("bank keeper is required for ante builder")
	}
	if options.SignModeHandler == nil {
		return nil, errors.New("sign mode handler is required for ante builder")
	}

	anteDecorators := []sdk.AnteDecorator{
		ante.NewSetUpContextDecorator(),
		circuitante.NewCircuitBreakerDecorator(options.CircuitKeeper),
		ante.NewExtensionOptionsDecorator(options.ExtensionOptionChecker),
		ante.NewValidateBasicDecorator(),
		ante.NewTxTimeoutHeightDecorator(),
		ante.NewValidateMemoDecorator(options.AccountKeeper),
		ante.NewConsumeGasForTxSizeDecorator(options.AccountKeeper),
		NewMonetaryFeeDecorator(), // check 0.3% fee before deduction
		ante.NewDeductFeeDecorator(options.AccountKeeper, options.BankKeeper, options.FeegrantKeeper, options.TxFeeChecker),
		ante.NewSetPubKeyDecorator(options.AccountKeeper),
		ante.NewValidateSigCountDecorator(options.AccountKeeper),
		ante.NewSigGasConsumeDecorator(options.AccountKeeper, options.SigGasConsumer),
		ante.NewSigVerificationDecorator(options.AccountKeeper, options.SignModeHandler),
		ante.NewIncrementSequenceDecorator(options.AccountKeeper),
		NewFeeRebateDecorator(options.CooperativeKeeper), // 80% rebate after deduction
	}

	return sdk.ChainAnteDecorators(anteDecorators...), nil
}
