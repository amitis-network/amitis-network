package types

import (
	"encoding/json"
	"errors"
	"fmt"
	"math/big"
	"strings"
)

// PoolPairKey returns a canonical string key for a token pair.
func PoolPairKey(denomA, denomB string) string {
	if denomA > denomB {
		denomA, denomB = denomB, denomA
	}
	return denomA + "/" + denomB
}

// NormalizeDecimals sorts two denoms; returns (smaller, larger) lexicographically.
func SortDenoms(d1, d2 string) (string, string) {
	if d1 > d2 {
		return d2, d1
	}
	return d1, d2
}

// --- Constant-product AMM math ---

// bigInt parses a decimal string to *big.Int. Panics on bad input (guard inputs before calling).
func bigInt(s string) *big.Int {
	n := new(big.Int)
	n.SetString(s, 10)
	return n
}

// ComputeSwapOutput computes the output amount for a constant-product swap.
//
//   outputAmount = (reserveOut * amountIn) / (reserveIn + amountIn)
//
// We apply a 0.3% fee: effective amountIn = amountIn * 997 / 1000
// Returns outputAmount as a string, and the new reserves (reserveIn, reserveOut) after the swap.
func ComputeSwapOutput(reserveIn, reserveOut, amountIn string) (outputAmount, newReserveIn, newReserveOut string, err error) {
	rIn := bigInt(reserveIn)
	rOut := bigInt(reserveOut)
	aIn := bigInt(amountIn)

	zero := big.NewInt(0)
	if rIn.Cmp(zero) <= 0 || rOut.Cmp(zero) <= 0 {
		return "", "", "", errors.New("reserves must be positive")
	}
	if aIn.Cmp(zero) <= 0 {
		return "", "", "", errors.New("amountIn must be positive")
	}

	// fee = 0.3% → numerator factor 997, denominator factor 1000
	aInWithFee := new(big.Int).Mul(aIn, big.NewInt(997))
	numerator := new(big.Int).Mul(aInWithFee, rOut)
	denominator := new(big.Int).Add(new(big.Int).Mul(rIn, big.NewInt(1000)), aInWithFee)

	out := new(big.Int).Div(numerator, denominator)
	if out.Cmp(rOut) >= 0 {
		return "", "", "", errors.New("insufficient liquidity")
	}

	nRin := new(big.Int).Add(rIn, aIn)
	nRout := new(big.Int).Sub(rOut, out)

	return out.String(), nRin.String(), nRout.String(), nil
}

// ComputeAddLiquidity computes shares to mint when adding liquidity.
// Returns (adjustedA, adjustedB, sharesToMint).
// On first liquidity: shares = sqrt(amountA * amountB), but we use a simpler initial = amountA for determinism.
func ComputeAddLiquidity(reserveA, reserveB, totalShares, amountA, amountB string) (adjA, adjB, shares string, err error) {
	rA := bigInt(reserveA)
	rB := bigInt(reserveB)
	ts := bigInt(totalShares)
	aA := bigInt(amountA)
	aB := bigInt(amountB)
	zero := big.NewInt(0)

	if aA.Cmp(zero) <= 0 || aB.Cmp(zero) <= 0 {
		return "", "", "", errors.New("amounts must be positive")
	}

	// Initial liquidity: no reserves yet
	if rA.Cmp(zero) == 0 && rB.Cmp(zero) == 0 {
		// Mint shares = geometric mean via integer sqrt of (amountA * amountB)
		product := new(big.Int).Mul(aA, aB)
		mintedShares := intSqrt(product)
		if mintedShares.Cmp(zero) == 0 {
			return "", "", "", errors.New("initial liquidity too small")
		}
		return amountA, amountB, mintedShares.String(), nil
	}

	// Proportional add — compute ideal amountB given amountA
	// idealB = amountA * reserveB / reserveA
	idealB := new(big.Int).Mul(aA, rB)
	idealB.Div(idealB, rA)

	var usedA, usedB *big.Int
	if idealB.Cmp(aB) <= 0 {
		// Use full amountA, proportional B
		usedA = aA
		usedB = idealB
	} else {
		// Use full amountB, compute proportional A
		idealA := new(big.Int).Mul(aB, rA)
		idealA.Div(idealA, rB)
		usedA = idealA
		usedB = aB
	}

	if usedA.Cmp(zero) <= 0 || usedB.Cmp(zero) <= 0 {
		return "", "", "", errors.New("computed contribution rounds to zero")
	}

	// Shares to mint = totalShares * usedA / reserveA
	mintedShares := new(big.Int).Mul(ts, usedA)
	mintedShares.Div(mintedShares, rA)
	if mintedShares.Cmp(zero) == 0 {
		return "", "", "", errors.New("liquidity too small: rounds to 0 shares")
	}

	return usedA.String(), usedB.String(), mintedShares.String(), nil
}

// ComputeRemoveLiquidity computes the token amounts returned when burning shares.
func ComputeRemoveLiquidity(reserveA, reserveB, totalShares, sharesToBurn string) (amountA, amountB string, err error) {
	rA := bigInt(reserveA)
	rB := bigInt(reserveB)
	ts := bigInt(totalShares)
	burn := bigInt(sharesToBurn)
	zero := big.NewInt(0)

	if burn.Cmp(zero) <= 0 {
		return "", "", errors.New("shares must be positive")
	}
	if burn.Cmp(ts) > 0 {
		return "", "", errors.New("shares exceed total supply")
	}

	outA := new(big.Int).Mul(burn, rA)
	outA.Div(outA, ts)
	outB := new(big.Int).Mul(burn, rB)
	outB.Div(outB, ts)

	if outA.Cmp(zero) == 0 || outB.Cmp(zero) == 0 {
		return "", "", errors.New("withdrawal too small: rounds to zero")
	}
	return outA.String(), outB.String(), nil
}

// intSqrt returns integer square root (floor).
func intSqrt(n *big.Int) *big.Int {
	if n.Sign() < 0 {
		return big.NewInt(0)
	}
	// Newton's method
	z := new(big.Int).Add(n, big.NewInt(1))
	z.Rsh(z, 1)
	x := new(big.Int).Set(n)
	for z.Cmp(x) < 0 {
		x.Set(z)
		t := new(big.Int).Div(n, z)
		t.Add(t, z)
		z.Rsh(t, 1)
	}
	return x
}

const BaseDenom = "uamts"

// ValidateCreatePool validates a MsgCreatePool — enforces AMTS on one side.
func ValidateCreatePool(m *MsgCreatePool) error {
	if m.Creator == "" {
		return errors.New("creator required")
	}
	if m.DenomA == "" || m.DenomB == "" {
		return errors.New("both denoms required")
	}
	if strings.EqualFold(m.DenomA, m.DenomB) {
		return errors.New("denoms must differ")
	}
	if m.DenomA != BaseDenom && m.DenomB != BaseDenom {
		return fmt.Errorf("one denom must be %s", BaseDenom)
	}
	aA := bigInt(m.AmountA)
	aB := bigInt(m.AmountB)
	if aA.Sign() <= 0 || aB.Sign() <= 0 {
		return fmt.Errorf("amounts must be positive, got %s and %s", m.AmountA, m.AmountB)
	}
	return nil
}

// --- Genesis ---

type GenesisState struct {
	Pools     []Pool              `json:"pools"`
	Positions []LiquidityPosition `json:"positions"`
	NextPoolId uint64             `json:"next_pool_id"`
}

func DefaultGenesis() *GenesisState {
	return &GenesisState{
		Pools:      []Pool{},
		Positions:  []LiquidityPosition{},
		NextPoolId: 1,
	}
}

func (gs *GenesisState) MarshalJSON() ([]byte, error) {
	type Alias GenesisState
	return json.Marshal((*Alias)(gs))
}
