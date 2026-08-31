package samts

import (
	"encoding/json"
	"net/http"

	"github.com/gorilla/mux"
	sdk "github.com/cosmos/cosmos-sdk/types"

	"github.com/amitis-network/amitis-network/x/samts/keeper"
)

func RegisterAPIRoutes(router *mux.Router, k keeper.Keeper, queryCtx func() (sdk.Context, error)) {

	// GET /amitis/samts/v1/params
	router.HandleFunc("/amitis/samts/v1/params", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		params := k.GetParams(ctx)
		writeJSON(w, map[string]interface{}{"params": params})
	}).Methods("GET")

	// GET /amitis/samts/v1/pool
	router.HandleFunc("/amitis/samts/v1/pool", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		total, err := k.TotalPool(ctx)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		maxLoan, err := k.MaxLoan(ctx)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		writeJSON(w, map[string]interface{}{
			"total_samts":     total.String(),
			"max_loan":        maxLoan.String(),
			"flash_loan_fee":  "0.05%",
		})
	}).Methods("GET")

	// GET /amitis/samts/v1/positions
	router.HandleFunc("/amitis/samts/v1/positions", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		positions, err := k.GetAllPositions(ctx)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		writeJSON(w, map[string]interface{}{"positions": positions})
	}).Methods("GET")
}

func writeJSON(w http.ResponseWriter, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, code int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{"error": msg})
}
