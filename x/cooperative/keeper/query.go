package keeper

import (
	"context"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// QueryTotalRebates returns the lifetime total rebates paid
func (k Keeper) QueryTotalRebates(ctx context.Context, req interface{}) (string, error) {
	if req == nil {
		return "", status.Error(codes.InvalidArgument, "invalid request")
	}
	total, err := k.GetTotalRebatesPaid(ctx)
	if err != nil {
		return "0", nil
	}
	return total, nil
}
