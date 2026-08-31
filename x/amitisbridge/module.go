package amitisbridge

import (
	"context"
	"encoding/json"
	"fmt"

	"cosmossdk.io/core/appmodule"
	"github.com/cosmos/cosmos-sdk/client"
	"github.com/cosmos/cosmos-sdk/codec"
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/cosmos/cosmos-sdk/types/module"
	"github.com/grpc-ecosystem/grpc-gateway/runtime"
	"github.com/spf13/cobra"
	"google.golang.org/grpc"

	"github.com/amitis-network/amitis-network/x/amitisbridge/keeper"
	"github.com/amitis-network/amitis-network/x/amitisbridge/types"
)

var (
	_ module.AppModuleBasic = AppModuleBasic{}
	_ appmodule.AppModule   = AppModule{}
	_ module.HasGenesis     = AppModule{}
)

type AppModuleBasic struct{}

func (AppModuleBasic) Name() string                                         { return types.ModuleName }
func (AppModuleBasic) RegisterLegacyAminoCodec(_ *codec.LegacyAmino)       {}
func (AppModuleBasic) RegisterInterfaces(_ codectypes.InterfaceRegistry)    {}
func (AppModuleBasic) RegisterGRPCGatewayRoutes(_ client.Context, _ *runtime.ServeMux) {}
func (AppModuleBasic) GetTxCmd() *cobra.Command                             { return nil }
func (AppModuleBasic) GetQueryCmd() *cobra.Command                          { return nil }

type AppModule struct {
	AppModuleBasic
	keeper keeper.Keeper
}

func NewAppModule(k keeper.Keeper) AppModule {
	return AppModule{keeper: k}
}

func (AppModule) IsOnePerModuleType() {}
func (AppModule) IsAppModule()        {}

// RegisterServices — no protobuf, msgs handled via direct keeper calls from REST
func (am AppModule) RegisterServices(_ grpc.ServiceRegistrar) error { return nil }

func (am AppModule) DefaultGenesis(_ codec.JSONCodec) json.RawMessage {
	bz, _ := json.Marshal(types.DefaultGenesisState())
	return bz
}

func (am AppModule) ValidateGenesis(_ codec.JSONCodec, _ client.TxEncodingConfig, bz json.RawMessage) error {
	var gs types.GenesisState
	return json.Unmarshal(bz, &gs)
}

func (am AppModule) InitGenesis(ctx sdk.Context, _ codec.JSONCodec, bz json.RawMessage) {
	var gs types.GenesisState
	if err := json.Unmarshal(bz, &gs); err != nil {
		panic(fmt.Sprintf("failed to unmarshal amitisbridge genesis: %s", err))
	}
	if err := am.keeper.SetParams(ctx, gs.Params); err != nil {
		panic(err)
	}
	for _, req := range gs.InboundRequests {
		if err := am.keeper.SetInboundRequest(ctx, req); err != nil {
			panic(err)
		}
	}
	for _, req := range gs.OutboundRequests {
		if err := am.keeper.SetOutboundRequest(ctx, req); err != nil {
			panic(err)
		}
	}
}

func (am AppModule) ExportGenesis(ctx sdk.Context, _ codec.JSONCodec) json.RawMessage {
	inbound, _ := am.keeper.GetAllInboundRequests(ctx)
	outbound, _ := am.keeper.GetAllOutboundRequests(ctx)
	if inbound == nil {
		inbound = []types.InboundRequest{}
	}
	if outbound == nil {
		outbound = []types.OutboundRequest{}
	}
	gs := types.GenesisState{
		Params:           am.keeper.GetParams(ctx),
		InboundRequests:  inbound,
		OutboundRequests: outbound,
	}
	bz, _ := json.Marshal(gs)
	return bz
}

func (am AppModule) ConsensusVersion() uint64 { return 1 }

func (am AppModule) BeginBlock(_ context.Context) error { return nil }

// EndBlock expires stale pending requests
func (am AppModule) EndBlock(ctx context.Context) error {
	am.keeper.ExpireStaleRequests(ctx)
	return nil
}
