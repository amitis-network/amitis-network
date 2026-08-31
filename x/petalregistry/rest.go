package petalregistry

import (
	"encoding/json"
	"net/http"
	"strconv"
	"strings"

	"github.com/gorilla/mux"
	"github.com/cosmos/cosmos-sdk/client"

	"github.com/amitis-network/amitis-network/x/petalregistry/types"
)

type petalJSON struct {
	Id               string `json:"id"`
	Owner            string `json:"owner"`
	ValidatorAddress string `json:"validator_address"`
	Name             string `json:"name"`
	Description      string `json:"description"`
	LifetimeRevenue  string `json:"lifetime_revenue"`
	PurchaseHeight   string `json:"purchase_height"`
}

func petalToJSON(p *types.Petal) petalJSON {
	return petalJSON{
		Id:               strconv.FormatUint(p.Id, 10),
		Owner:            p.Owner,
		ValidatorAddress: p.ValidatorAddress,
		Name:             p.Name,
		Description:      p.Description,
		LifetimeRevenue:  "0",
		PurchaseHeight:   strconv.FormatInt(p.PurchaseHeight, 10),
	}
}

func writeJSON(w http.ResponseWriter, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(v)
}

// RegisterAPIRoutes registers petalregistry REST handlers on the gorilla/mux router
func RegisterAPIRoutes(router *mux.Router, clientCtx client.Context) {

	// GET /amitis/petalregistry/v1/petals
	router.HandleFunc("/amitis/petalregistry/v1/petals", func(w http.ResponseWriter, r *http.Request) {
		queryClient := types.NewQueryClient(clientCtx)
		res, err := queryClient.Petals(r.Context(), &types.QueryPetalsRequest{})
		if err != nil {
			http.Error(w, `{"error":"`+err.Error()+`"}`, http.StatusInternalServerError)
			return
		}
		var list []petalJSON
		for _, p := range res.Petals {
			list = append(list, petalToJSON(p))
		}
		if list == nil {
			list = []petalJSON{}
		}
		writeJSON(w, map[string]interface{}{"petals": list})
	}).Methods("GET")

	// GET /amitis/petalregistry/v1/petal/{id}
	router.HandleFunc("/amitis/petalregistry/v1/petal/{id}", func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id, err := strconv.ParseUint(vars["id"], 10, 64)
		if err != nil {
			http.Error(w, `{"error":"invalid petal id"}`, http.StatusBadRequest)
			return
		}
		queryClient := types.NewQueryClient(clientCtx)
		res, err := queryClient.Petal(r.Context(), &types.QueryPetalRequest{Id: id})
		if err != nil {
			http.Error(w, `{"error":"petal not found"}`, http.StatusNotFound)
			return
		}
		writeJSON(w, map[string]interface{}{"petal": petalToJSON(res.Petal)})
	}).Methods("GET")

	// GET /amitis/petalregistry/v1/petals/owner/{owner}
	router.HandleFunc("/amitis/petalregistry/v1/petals/owner/{owner}", func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		owner := vars["owner"]
		if !strings.HasPrefix(owner, "amitis") {
			http.Error(w, `{"error":"invalid owner address"}`, http.StatusBadRequest)
			return
		}
		queryClient := types.NewQueryClient(clientCtx)
		res, err := queryClient.PetalsByOwner(r.Context(), &types.QueryPetalsByOwnerRequest{Owner: owner})
		if err != nil {
			http.Error(w, `{"error":"failed to query petals"}`, http.StatusInternalServerError)
			return
		}
		var list []petalJSON
		for _, p := range res.Petals {
			list = append(list, petalToJSON(p))
		}
		if list == nil {
			list = []petalJSON{}
		}
		writeJSON(w, map[string]interface{}{"petals": list})
	}).Methods("GET")
}
