package types

const (
	ModuleName = "samts"
	StoreKey   = "samts"
	RouterKey  = "samts"

	SAMTSDenom = "usamts"

	DefaultFlashLoanFeeRate = "0.0005"
	DefaultMaxLoanFraction  = "0.5"
)

var (
	KeyPosition  = []byte{0x01}
	KeyParams    = []byte{0x02}
	KeyPoolState = []byte{0x03}
)

type GenesisState struct {
	Positions []ValidatorPosition `json:"positions"`
	Params    Params              `json:"params"`
}
