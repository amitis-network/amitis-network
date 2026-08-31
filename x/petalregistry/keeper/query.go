package keeper

import (
	"context"

	"github.com/amitis-network/amitis-network/x/petalregistry/types"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

var _ types.QueryServer = queryServer{}

// queryServer implements the Query gRPC service
type queryServer struct {
	types.UnimplementedQueryServer
	k Keeper
}

// NewQueryServer returns an implementation of the QueryServer interface
func NewQueryServer(keeper Keeper) types.QueryServer {
	return &queryServer{k: keeper}
}

// Petal queries a petal by ID
func (q queryServer) Petal(ctx context.Context, req *types.QueryPetalRequest) (*types.QueryPetalResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "invalid request")
	}

	petal, err := q.k.GetPetal(ctx, req.Id)
	if err != nil {
		return nil, status.Errorf(codes.NotFound, "petal %d not found", req.Id)
	}

	return &types.QueryPetalResponse{Petal: petal}, nil
}

// Petals queries all petals
func (q queryServer) Petals(ctx context.Context, req *types.QueryPetalsRequest) (*types.QueryPetalsResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "invalid request")
	}

	petals, err := q.k.GetAllPetals(ctx)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to query petals: %s", err)
	}

	return &types.QueryPetalsResponse{Petals: petals}, nil
}

// PetalsByOwner queries petals owned by an address
func (q queryServer) PetalsByOwner(ctx context.Context, req *types.QueryPetalsByOwnerRequest) (*types.QueryPetalsByOwnerResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "invalid request")
	}
	if req.Owner == "" {
		return nil, status.Error(codes.InvalidArgument, "owner address cannot be empty")
	}

	petals, err := q.k.GetPetalsByOwner(ctx, req.Owner)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to query petals by owner: %s", err)
	}

	return &types.QueryPetalsByOwnerResponse{Petals: petals}, nil
}
