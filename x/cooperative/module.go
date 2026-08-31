package cooperative

import (
	"encoding/json"
	"fmt"

	"cosmossdk.io/core/appmodule"
	"github.com/cosmos/cosmos-sdk/codec"
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/cosmos/cosmos-sdk/types/module"
	"github.com/grpc-ecosystem/grpc-gateway/runtime"
	"github.com/spf13/cobra"
	"google.golang.org/grpc"

	"github.com/amitis-network/amitis-network/x/cooperative/keeper"
	"github.com/amitis-network/amitis-network/x/cooperative/types"

	"github.com/cosmos/cosmos-sdk/client"
)

var (
	_ module.AppModuleBasic = AppModuleBasic{}
	_ appmodule.AppModule   = AppModule{}
)

// AppModuleBasic defines the basic application module for cooperative
type AppModuleBasic struct{}

func (AppModuleBasic) Name() string { return types.ModuleName }

func (AppModuleBasic) RegisterLegacyAminoCodec(_ *codec.LegacyAmino) {}

func (AppModuleBasic) RegisterInterfaces(_ codectypes.InterfaceRegistry) {}

func (AppModuleBasic) DefaultGenesis(_ codec.JSONCodec) json.RawMessage {
	gs := types.DefaultGenesisState()
	bz, err := json.Marshal(gs)
	if err != nil {
		panic(fmt.Sprintf("failed to marshal cooperative genesis state: %s", err))
	}
	return bz
}

func (AppModuleBasic) ValidateGenesis(_ codec.JSONCodec, _ client.TxEncodingConfig, bz json.RawMessage) error {
	var gs types.GenesisState
	return json.Unmarshal(bz, &gs)
}

func (AppModuleBasic) RegisterGRPCGatewayRoutes(_ client.Context, _ *runtime.ServeMux) {}

func (AppModuleBasic) GetQueryCmd() *cobra.Command { return nil }
func (AppModuleBasic) GetTxCmd() *cobra.Command    { return nil }

// AppModule implements the cooperative application module
type AppModule struct {
	AppModuleBasic
	keeper keeper.Keeper
}

func NewAppModule(keeper keeper.Keeper) AppModule {
	return AppModule{keeper: keeper}
}

func (AppModule) IsOnePerModuleType() {}
func (AppModule) IsAppModule()        {}

func (am AppModule) InitGenesis(ctx sdk.Context, _ codec.JSONCodec, bz json.RawMessage) {
	var gs types.GenesisState
	if err := json.Unmarshal(bz, &gs); err != nil {
		panic(fmt.Sprintf("failed to unmarshal cooperative genesis: %s", err))
	}
	if err := am.keeper.SetTotalRebatesPaid(ctx, gs.TotalRebatesPaid); err != nil {
		panic(fmt.Sprintf("failed to set cooperative genesis state: %s", err))
	}
}

func (am AppModule) ExportGenesis(ctx sdk.Context, _ codec.JSONCodec) json.RawMessage {
	total, err := am.keeper.GetTotalRebatesPaid(ctx)
	if err != nil {
		total = "0"
	}
	gs := &types.GenesisState{TotalRebatesPaid: total}
	bz, err := json.Marshal(gs)
	if err != nil {
		panic(fmt.Sprintf("failed to marshal cooperative genesis: %s", err))
	}
	return bz
}

func (AppModule) ConsensusVersion() uint64 { return 1 }

func (AppModule) RegisterServices(_ grpc.ServiceRegistrar) error { return nil }
