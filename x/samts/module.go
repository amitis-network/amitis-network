package samts

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
	"google.golang.org/grpc"
	"github.com/spf13/cobra"

	"github.com/amitis-network/amitis-network/x/samts/keeper"
	"github.com/amitis-network/amitis-network/x/samts/types"
)

var (
	_ module.AppModuleBasic = AppModuleBasic{}
	_ appmodule.AppModule   = AppModule{}
	_ module.HasGenesis     = AppModule{}
)

type AppModuleBasic struct{}

func (AppModuleBasic) Name() string { return types.ModuleName }
func (AppModuleBasic) RegisterLegacyAminoCodec(_ *codec.LegacyAmino) {}
func (AppModuleBasic) RegisterInterfaces(registry codectypes.InterfaceRegistry) {
	registry.RegisterImplementations(
		(*sdk.Msg)(nil),
		&types.MsgFlashLoan{},
	)
}
func (AppModuleBasic) RegisterGRPCGatewayRoutes(_ client.Context, _ *runtime.ServeMux) {}
func (AppModuleBasic) GetTxCmd() *cobra.Command                                         { return nil }
func (AppModuleBasic) GetQueryCmd() *cobra.Command                                      { return nil }

type AppModule struct {
	AppModuleBasic
	keeper keeper.Keeper
}

func NewAppModule(k keeper.Keeper) AppModule {
	return AppModule{keeper: k}
}

func (AppModule) IsOnePerModuleType() {}
func (AppModule) IsAppModule()        {}

func (am AppModule) RegisterServices(registrar grpc.ServiceRegistrar) error {
	types.RegisterMsgServer(registrar, keeper.NewMsgServer(am.keeper))
	return nil
}

func (am AppModule) DefaultGenesis(_ codec.JSONCodec) json.RawMessage {
	gs := types.GenesisState{
		Positions: []types.ValidatorPosition{},
		Params: types.Params{
			FlashLoanFeeRate: types.DefaultFlashLoanFeeRate,
			MaxLoanFraction:  types.DefaultMaxLoanFraction,
		},
	}
	bz, _ := json.Marshal(gs)
	return bz
}

func (am AppModule) ValidateGenesis(_ codec.JSONCodec, _ client.TxEncodingConfig, bz json.RawMessage) error {
	var gs types.GenesisState
	return json.Unmarshal(bz, &gs)
}

func (am AppModule) InitGenesis(ctx sdk.Context, _ codec.JSONCodec, bz json.RawMessage) {
	var gs types.GenesisState
	if err := json.Unmarshal(bz, &gs); err != nil {
		panic(fmt.Sprintf("failed to unmarshal samts genesis: %s", err))
	}
	if err := am.keeper.SetParams(ctx, gs.Params); err != nil {
		panic(err)
	}
	for _, pos := range gs.Positions {
		if err := am.keeper.SetPosition(ctx, pos); err != nil {
			panic(err)
		}
	}
}

func (am AppModule) ExportGenesis(ctx sdk.Context, _ codec.JSONCodec) json.RawMessage {
	positions, _ := am.keeper.GetAllPositions(ctx)
	if positions == nil {
		positions = []types.ValidatorPosition{}
	}
	gs := types.GenesisState{
		Positions: positions,
		Params:    am.keeper.GetParams(ctx),
	}
	bz, _ := json.Marshal(gs)
	return bz
}

func (am AppModule) ConsensusVersion() uint64 { return 1 }

func (am AppModule) BeginBlock(_ context.Context) error { return nil }
func (am AppModule) EndBlock(_ context.Context) error   { return nil }
