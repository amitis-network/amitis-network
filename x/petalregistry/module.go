package petalregistry

import (
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

	"github.com/amitis-network/amitis-network/x/petalregistry/client/cli"
	"github.com/amitis-network/amitis-network/x/petalregistry/keeper"
	"github.com/amitis-network/amitis-network/x/petalregistry/types"
)

var (
	_ module.AppModuleBasic      = AppModuleBasic{}
	_ appmodule.AppModule         = AppModule{}
	_ appmodule.HasServices      = AppModule{}
)

// AppModuleBasic defines the basic application module used by the petalregistry module.
type AppModuleBasic struct {
	cdc codec.Codec
}

func (AppModuleBasic) Name() string { return types.ModuleName }

func (AppModuleBasic) RegisterLegacyAminoCodec(cdc *codec.LegacyAmino) {}

func (AppModuleBasic) RegisterInterfaces(registry codectypes.InterfaceRegistry) {
	registry.RegisterImplementations(
		(*sdk.Msg)(nil),
		&types.MsgTransferPetal{},
		&types.MsgUpdatePetalMetadata{},
	)
}

func (AppModuleBasic) DefaultGenesis(cdc codec.JSONCodec) json.RawMessage {
	return cdc.MustMarshalJSON(&types.GenesisState{})
}

func (AppModuleBasic) ValidateGenesis(_ codec.JSONCodec, _ client.TxEncodingConfig, bz json.RawMessage) error {
	var raw map[string]json.RawMessage
	return json.Unmarshal(bz, &raw)
}

func (amb AppModuleBasic) RegisterGRPCGatewayRoutes(_ client.Context, _ *runtime.ServeMux) {
}

func (AppModuleBasic) GetQueryCmd() *cobra.Command {
	return cli.GetQueryCmd()
}

func (AppModuleBasic) GetTxCmd() *cobra.Command {
	return cli.GetTxCmd()
}

// AppModule implements an application module for the petalregistry module.
type AppModule struct {
	AppModuleBasic
	keeper keeper.Keeper
}

func NewAppModule(cdc codec.Codec, keeper keeper.Keeper) AppModule {
	return AppModule{
		AppModuleBasic: AppModuleBasic{cdc: cdc},
		keeper:         keeper,
	}
}

func (AppModule) IsOnePerModuleType() {}
func (AppModule) IsAppModule()        {}

func (am AppModule) InitGenesis(ctx sdk.Context, cdc codec.JSONCodec, data json.RawMessage) {
	// Use standard JSON unmarshal to avoid proto customtype issues with math.Int
	var rawState struct {
		Petals []struct {
			Id               string `json:"id"`
			Owner            string `json:"owner"`
			ValidatorAddress string `json:"validator_address"`
			Name             string `json:"name"`
			Description      string `json:"description"`
		} `json:"petals"`
	}
	if err := json.Unmarshal(data, &rawState); err != nil {
		panic(fmt.Sprintf("failed to unmarshal petalregistry genesis: %s", err))
	}

	for _, raw := range rawState.Petals {
		var id uint64
		fmt.Sscanf(raw.Id, "%d", &id)
		petal := &types.Petal{
			Id:               id,
			Owner:            raw.Owner,
			ValidatorAddress: raw.ValidatorAddress,
			Name:             raw.Name,
			Description:      raw.Description,
		}
		if err := am.keeper.SetPetal(ctx, petal); err != nil {
			panic(fmt.Sprintf("failed to set petal %d in genesis: %s", petal.Id, err))
		}
	}
}

func (am AppModule) ExportGenesis(ctx sdk.Context, cdc codec.JSONCodec) json.RawMessage {
	petals, err := am.keeper.GetAllPetals(ctx)
	if err != nil {
		panic(fmt.Sprintf("failed to export petals: %s", err))
	}
	gs := &types.GenesisState{
		Petals: func() []types.Petal { r := make([]types.Petal, len(petals)); for i, p := range petals { r[i] = *p }; return r }(),
		NextPetalId: 22, // fixed 21 validators
	}
	return cdc.MustMarshalJSON(gs)
}

func (am AppModule) ConsensusVersion() uint64 { return 1 }

func (am AppModule) RegisterServices(registrar grpc.ServiceRegistrar) error {
	types.RegisterQueryServer(registrar, keeper.NewQueryServer(am.keeper))
	types.RegisterMsgServer(registrar, keeper.NewMsgServer(am.keeper))
	return nil
}
