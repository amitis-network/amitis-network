package types

import "context"

// ValidatorPosition tracks a liquid staking position for a validator
type ValidatorPosition struct {
	ValidatorAddress string `json:"validator_address"`
	SamtsBalance     string `json:"samts_balance"`
	AMTSLocked       string `json:"amts_locked"`
	Owner            string `json:"owner"`
}

// Params defines the parameters for the samts module
type Params struct {
	FlashLoanFeeRate string `json:"flash_loan_fee_rate"`
	MaxLoanFraction  string `json:"max_loan_fraction"`
}

func DefaultParams() Params {
	return Params{
		FlashLoanFeeRate: DefaultFlashLoanFeeRate,
		MaxLoanFraction:  DefaultMaxLoanFraction,
	}
}

// MsgFlashLoan is the message to execute a flash loan
type MsgFlashLoan struct {
	Borrower string `json:"borrower"`
	Amount   string `json:"amount"`
}

// Proto interface methods required by cosmos SDK
func (m *MsgFlashLoan) ProtoMessage()             {}
func (m *MsgFlashLoan) Reset()                    { *m = MsgFlashLoan{} }
func (m *MsgFlashLoan) String() string            { return m.Borrower }
func (m *MsgFlashLoan) ProtoSize() int            { return 0 }
func (m *MsgFlashLoan) Marshal() ([]byte, error)  { return []byte{}, nil }
func (m *MsgFlashLoan) Unmarshal([]byte) error    { return nil }

// MsgFlashLoanResponse is the response to a flash loan
type MsgFlashLoanResponse struct {
	AmountBorrowed string `json:"amount_borrowed"`
	Fee            string `json:"fee"`
}

func (m *MsgFlashLoanResponse) ProtoMessage()             {}
func (m *MsgFlashLoanResponse) Reset()                    { *m = MsgFlashLoanResponse{} }
func (m *MsgFlashLoanResponse) String() string            { return m.AmountBorrowed }

// MsgServer defines the samts message server interface
type MsgServer interface {
	FlashLoan(ctx context.Context, msg *MsgFlashLoan) (*MsgFlashLoanResponse, error)
}

// UnimplementedMsgServer is a stub for the MsgServer interface
type UnimplementedMsgServer struct{}

func (UnimplementedMsgServer) FlashLoan(ctx context.Context, msg *MsgFlashLoan) (*MsgFlashLoanResponse, error) {
	return nil, nil
}

// RegisterMsgServer is a no-op stub since we use REST not gRPC
func RegisterMsgServer(_ interface{}, _ MsgServer) {}
