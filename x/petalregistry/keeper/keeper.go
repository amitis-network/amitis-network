package keeper

import (
	"context"
	"encoding/json"
	"fmt"

	"cosmossdk.io/collections"
	"cosmossdk.io/core/store"
	"github.com/cosmos/cosmos-sdk/codec"

	"github.com/amitis-network/amitis-network/x/petalregistry/types"
)

type Keeper struct {
	cdc          codec.BinaryCodec
	storeService store.KVStoreService
	Petals       collections.Map[uint64, []byte]
}

func NewKeeper(
	cdc codec.BinaryCodec,
	storeService store.KVStoreService,
) Keeper {
	sb := collections.NewSchemaBuilder(storeService)
	return Keeper{
		cdc:          cdc,
		storeService: storeService,
		Petals:       collections.NewMap(sb, collections.NewPrefix(1), "petals", collections.Uint64Key, collections.BytesValue),
	}
}

// petalStore is a flat JSON-serializable representation of a Petal
type petalStore struct {
	Id               uint64 `json:"id"`
	Owner            string `json:"owner"`
	ValidatorAddress string `json:"validator_address"`
	Name             string `json:"name"`
	Description      string `json:"description"`
	LifetimeRevenue  string `json:"lifetime_revenue"`
	PurchaseHeight   int64  `json:"purchase_height"`
}

func petalToStore(p *types.Petal) petalStore {
	return petalStore{
		Id:               p.Id,
		Owner:            p.Owner,
		ValidatorAddress: p.ValidatorAddress,
		Name:             p.Name,
		Description:      p.Description,
		LifetimeRevenue:  "0",
		PurchaseHeight:   p.PurchaseHeight,
	}
}

func storeToProtoPetal(s petalStore) *types.Petal {
	return &types.Petal{
		Id:               s.Id,
		Owner:            s.Owner,
		ValidatorAddress: s.ValidatorAddress,
		Name:             s.Name,
		Description:      s.Description,
		PurchaseHeight:   s.PurchaseHeight,
	}
}

// SetPetal stores a petal using plain JSON
func (k Keeper) SetPetal(ctx context.Context, petal *types.Petal) error {
	bz, err := json.Marshal(petalToStore(petal))
	if err != nil {
		return fmt.Errorf("failed to marshal petal: %w", err)
	}
	return k.Petals.Set(ctx, petal.Id, bz)
}

// GetPetal retrieves a petal by ID
func (k Keeper) GetPetal(ctx context.Context, id uint64) (*types.Petal, error) {
	bz, err := k.Petals.Get(ctx, id)
	if err != nil {
		return nil, fmt.Errorf("petal %d not found: %w", id, err)
	}
	var s petalStore
	if err := json.Unmarshal(bz, &s); err != nil {
		return nil, fmt.Errorf("failed to unmarshal petal: %w", err)
	}
	return storeToProtoPetal(s), nil
}

// HasPetal checks if a petal exists
func (k Keeper) HasPetal(ctx context.Context, id uint64) (bool, error) {
	return k.Petals.Has(ctx, id)
}

// GetAllPetals returns all petals
func (k Keeper) GetAllPetals(ctx context.Context) ([]*types.Petal, error) {
	var petals []*types.Petal
	err := k.Petals.Walk(ctx, nil, func(id uint64, bz []byte) (bool, error) {
		var s petalStore
		if err := json.Unmarshal(bz, &s); err != nil {
			return true, fmt.Errorf("failed to unmarshal petal %d: %w", id, err)
		}
		petals = append(petals, storeToProtoPetal(s))
		return false, nil
	})
	return petals, err
}

// GetPetalsByOwner returns all petals owned by the given address
func (k Keeper) GetPetalsByOwner(ctx context.Context, owner string) ([]*types.Petal, error) {
	all, err := k.GetAllPetals(ctx)
	if err != nil {
		return nil, err
	}
	var result []*types.Petal
	for _, p := range all {
		if p.Owner == owner {
			result = append(result, p)
		}
	}
	return result, nil
}

// TransferPetal transfers ownership of a petal to a new address.
// Only the current owner may call this.
func (k Keeper) TransferPetal(ctx context.Context, msg *types.MsgTransferPetal) (*types.MsgTransferPetalResponse, error) {
	petal, err := k.GetPetal(ctx, msg.PetalId)
	if err != nil {
		return nil, fmt.Errorf("petal %d not found", msg.PetalId)
	}
	if petal.Owner != msg.Owner {
		return nil, fmt.Errorf("unauthorized: %s does not own petal %d", msg.Owner, msg.PetalId)
	}
	if msg.NewOwner == "" {
		return nil, fmt.Errorf("new_owner cannot be empty")
	}
	if msg.NewOwner == msg.Owner {
		return nil, fmt.Errorf("new_owner must differ from current owner")
	}
	petal.Owner = msg.NewOwner
	if err := k.SetPetal(ctx, petal); err != nil {
		return nil, fmt.Errorf("failed to update petal: %w", err)
	}
	return &types.MsgTransferPetalResponse{
		PetalId:  msg.PetalId,
		NewOwner: msg.NewOwner,
	}, nil
}
