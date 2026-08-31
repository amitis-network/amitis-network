package keeper

import (
	"context"

	"github.com/amitis-network/amitis-network/x/petalregistry/types"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

var _ types.MsgServer = msgServer{}

type msgServer struct {
	types.UnimplementedMsgServer
	k Keeper
}

// NewMsgServer returns an implementation of the MsgServer interface
func NewMsgServer(keeper Keeper) types.MsgServer {
	return &msgServer{k: keeper}
}

// UpdatePetalMetadata allows the owner to update a petal's name and description
func (m msgServer) UpdatePetalMetadata(ctx context.Context, req *types.MsgUpdatePetalMetadata) (*types.MsgUpdatePetalMetadataResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "invalid request")
	}
	if req.Owner == "" {
		return nil, status.Error(codes.InvalidArgument, "owner cannot be empty")
	}

	petal, err := m.k.GetPetal(ctx, req.PetalId)
	if err != nil {
		return nil, status.Errorf(codes.NotFound, "petal %d not found", req.PetalId)
	}

	if petal.Owner != req.Owner {
		return nil, status.Errorf(codes.PermissionDenied,
			"unauthorized: %s is not the owner of petal %d (owner is %s)",
			req.Owner, req.PetalId, petal.Owner)
	}

	if len(req.Name) > 64 {
		return nil, status.Error(codes.InvalidArgument, "name exceeds 64 character limit")
	}
	if len(req.Description) > 512 {
		return nil, status.Error(codes.InvalidArgument, "description exceeds 512 character limit")
	}

	petal.Name = req.Name
	petal.Description = req.Description

	if err := m.k.SetPetal(ctx, petal); err != nil {
		return nil, status.Errorf(codes.Internal, "failed to update petal: %s", err)
	}

	return &types.MsgUpdatePetalMetadataResponse{}, nil
}

// TransferPetal transfers ownership of a petal to a new address
func (m msgServer) TransferPetal(ctx context.Context, req *types.MsgTransferPetal) (*types.MsgTransferPetalResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "invalid request")
	}
	resp, err := m.k.TransferPetal(ctx, req)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "%s", err)
	}
	return resp, nil
}
