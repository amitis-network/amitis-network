package amitisdex

import (
	"encoding/json"

	"cosmossdk.io/core/appmodule"
	"github.com/cosmos/cosmos-sdk/client"
	"github.com/cosmos/cosmos-sdk/codec"
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/cosmos/cosmos-sdk/types/module"
	"github.com/grpc-ecosystem/grpc-gateway/runtime"
	"github.com/spf13/cobra"

	"github.com/amitis-network/amitis-network/x/amitisdex/keeper"
	"github.com/amitis-network/amitis-network/x/amitisdex/types"
)

var (
	_ module.AppModuleBasic = AppModuleBasic{}
	_ appmodule.AppModule   = AppModule{}
)

// AppModuleBasic defines the basic application module.
type AppModuleBasic struct {
	cdc codec.Codec
}

func (AppModuleBasic) Name() string                                                    { return types.ModuleName }
func (AppModuleBasic) RegisterLegacyAminoCodec(_ *codec.LegacyAmino)  {}
func (AppModuleBasic) RegisterInterfaces(registry codectypes.InterfaceRegistry) {
	registry.RegisterImplementations(
		(*sdk.Msg)(nil),
		&types.MsgCreatePool{},
		&types.MsgAddLiquidity{},
		&types.MsgRemoveLiquidity{},
		&types.MsgSwapExactIn{},
	)
}
func (AppModuleBasic) RegisterGRPCGatewayRoutes(_ client.Context, _ *runtime.ServeMux) {}
func (AppModuleBasic) GetQueryCmd() *cobra.Command                                     { return nil }
func (AppModuleBasic) GetTxCmd() *cobra.Command                                        { return nil }

func (AppModuleBasic) DefaultGenesis(_ codec.JSONCodec) json.RawMessage {
	bz, _ := json.Marshal(types.DefaultGenesis())
	return bz
}

func (AppModuleBasic) ValidateGenesis(_ codec.JSONCodec, _ client.TxEncodingConfig, bz json.RawMessage) error {
	var gs types.GenesisState
	return json.Unmarshal(bz, &gs)
}

// AppModule implements the application module.
type AppModule struct {
	AppModuleBasic
	keeper keeper.Keeper
}

func NewAppModule(cdc codec.Codec, k keeper.Keeper) AppModule {
	return AppModule{
		AppModuleBasic: AppModuleBasic{cdc: cdc},
		keeper:         k,
	}
}

func (AppModule) IsOnePerModuleType() {}
func (AppModule) IsAppModule()        {}

func (am AppModule) RegisterServices(cfg module.Configurator) {
	types.RegisterMsgServer(cfg.MsgServer(), keeper.NewMsgServer(am.keeper))
}

func (am AppModule) InitGenesis(ctx sdk.Context, _ codec.JSONCodec, data json.RawMessage) {
	var gs types.GenesisState
	if err := json.Unmarshal(data, &gs); err != nil {
		panic("amitisdex InitGenesis: failed to unmarshal: " + err.Error())
	}
	for i := range gs.Pools {
		if err := am.keeper.SetPool(ctx, &gs.Pools[i]); err != nil {
			panic("amitisdex InitGenesis: set pool: " + err.Error())
		}
	}
	for i := range gs.Positions {
		if err := am.keeper.SetPosition(ctx, &gs.Positions[i]); err != nil {
			panic("amitisdex InitGenesis: set position: " + err.Error())
		}
	}
	if gs.NextPoolId > 0 {
		if err := am.keeper.NextPoolId.Set(ctx, gs.NextPoolId); err != nil {
			panic("amitisdex InitGenesis: set next pool id: " + err.Error())
		}
	}
}

func (am AppModule) ExportGenesis(ctx sdk.Context, _ codec.JSONCodec) json.RawMessage {
	pools, _ := am.keeper.GetAllPools(ctx)
	positions, _ := am.keeper.GetAllPositions(ctx)
	nextId, _ := am.keeper.NextPoolId.Get(ctx)

	poolVals := make([]types.Pool, len(pools))
	for i, p := range pools {
		poolVals[i] = *p
	}
	posVals := make([]types.LiquidityPosition, len(positions))
	for i, p := range positions {
		posVals[i] = *p
	}

	gs := types.GenesisState{
		Pools:      poolVals,
		Positions:  posVals,
		NextPoolId: nextId,
	}
	bz, _ := json.Marshal(gs)
	return bz
}

func (am AppModule) ConsensusVersion() uint64 { return 1 }
