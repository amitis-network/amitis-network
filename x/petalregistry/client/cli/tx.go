package cli

import (
	"strconv"

	"github.com/cosmos/cosmos-sdk/client"
	"github.com/cosmos/cosmos-sdk/client/flags"
	"github.com/cosmos/cosmos-sdk/client/tx"
	"github.com/spf13/cobra"

	"github.com/amitis-network/amitis-network/x/petalregistry/types"
)

func GetTxCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:                        types.ModuleName,
		Short:                      "Petalregistry transaction subcommands",
		DisableFlagParsing:         true,
		SuggestionsMinimumDistance: 2,
		RunE:                       client.ValidateCmd,
	}
	cmd.AddCommand(
		CmdTransferPetal(),
		CmdUpdatePetalMetadata(),
	)
	return cmd
}

func CmdTransferPetal() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "transfer-petal [petal-id] [new-owner]",
		Short: "Transfer ownership of a petal to a new address",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			clientCtx, err := client.GetClientTxContext(cmd)
			if err != nil {
				return err
			}
			petalId, err := strconv.ParseUint(args[0], 10, 64)
			if err != nil {
				return err
			}
			msg := &types.MsgTransferPetal{
				Owner:    clientCtx.GetFromAddress().String(),
				PetalId:  petalId,
				NewOwner: args[1],
			}
			return tx.GenerateOrBroadcastTxCLI(clientCtx, cmd.Flags(), msg)
		},
	}
	flags.AddTxFlagsToCmd(cmd)
	return cmd
}

func CmdUpdatePetalMetadata() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "update-metadata [petal-id] [name] [description]",
		Short: "Update the name and description of a petal you own",
		Args:  cobra.ExactArgs(3),
		RunE: func(cmd *cobra.Command, args []string) error {
			clientCtx, err := client.GetClientTxContext(cmd)
			if err != nil {
				return err
			}
			petalId, err := strconv.ParseUint(args[0], 10, 64)
			if err != nil {
				return err
			}
			msg := &types.MsgUpdatePetalMetadata{
				Owner:       clientCtx.GetFromAddress().String(),
				PetalId:     petalId,
				Name:        args[1],
				Description: args[2],
			}
			return tx.GenerateOrBroadcastTxCLI(clientCtx, cmd.Flags(), msg)
		},
	}
	flags.AddTxFlagsToCmd(cmd)
	return cmd
}
