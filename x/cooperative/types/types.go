package types

const (
	ModuleName = "cooperative"
	StoreKey   = ModuleName

	// RebateNumerator / RebateDenominator = 80% rebate to tx senders
	RebateNumerator   = uint64(80)
	RebateDenominator = uint64(100)

	// ValidatorShareNumerator / ValidatorShareDenominator = 20% to validators (split across 21)
	ValidatorShareNumerator   = uint64(20)
	ValidatorShareDenominator = uint64(100)

	// AttributeKeyRebateAmount is the event attribute for rebate amount
	AttributeKeyRebateAmount  = "rebate_amount"
	AttributeKeyRebateAddress = "rebate_address"
	EventTypeRebate           = "fee_rebate"
)

// GenesisState defines the cooperative module genesis state
type GenesisState struct {
	// TotalRebatesPaid tracks lifetime rebates paid out (in uamts)
	TotalRebatesPaid string `json:"total_rebates_paid"`
}

func DefaultGenesisState() *GenesisState {
	return &GenesisState{
		TotalRebatesPaid: "0",
	}
}

func (gs *GenesisState) Reset()         {}
func (gs *GenesisState) String() string  { return gs.TotalRebatesPaid }
func (gs *GenesisState) ProtoMessage()   {}
