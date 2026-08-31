package amitisdex

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/gorilla/mux"
	sdk "github.com/cosmos/cosmos-sdk/types"

	"github.com/amitis-network/amitis-network/x/amitisdex/keeper"
	"github.com/amitis-network/amitis-network/x/amitisdex/types"
)

func writeDEXJSON(w http.ResponseWriter, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(v)
}

func writeDEXError(w http.ResponseWriter, code int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{"error": msg})
}

// RegisterAPIRoutes registers amitisdex REST handlers.
// queryCtx is a function that returns a fresh sdk.Context for read queries —
// pass app.BaseApp.CreateQueryContext(0, false) from app.go.
func RegisterAPIRoutes(router *mux.Router, k keeper.Keeper, queryCtx func() (sdk.Context, error)) {

	// GET /amitis/amitisdex/v1/pools
	router.HandleFunc("/amitis/amitisdex/v1/pools", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeDEXError(w, http.StatusServiceUnavailable, "node not ready: "+err.Error())
			return
		}
		pools, err := k.GetAllPools(ctx)
		if err != nil {
			writeDEXError(w, http.StatusInternalServerError, err.Error())
			return
		}
		if pools == nil {
			pools = []*types.Pool{}
		}
		writeDEXJSON(w, map[string]interface{}{"pools": pools})
	}).Methods("GET")

	// GET /amitis/amitisdex/v1/pools/{id}
	router.HandleFunc("/amitis/amitisdex/v1/pools/{id}", func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		id, err := strconv.ParseUint(vars["id"], 10, 64)
		if err != nil {
			writeDEXError(w, http.StatusBadRequest, "invalid pool id")
			return
		}
		ctx, err := queryCtx()
		if err != nil {
			writeDEXError(w, http.StatusServiceUnavailable, "node not ready: "+err.Error())
			return
		}
		pool, err := k.GetPool(ctx, id)
		if err != nil {
			writeDEXError(w, http.StatusNotFound, fmt.Sprintf("pool %d not found", id))
			return
		}
		writeDEXJSON(w, map[string]interface{}{"pool": pool})
	}).Methods("GET")

	// GET /amitis/amitisdex/v1/quote?pool_id=1&denom_in=uamts&amount_in=1000000
	router.HandleFunc("/amitis/amitisdex/v1/quote", func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		poolIdStr := q.Get("pool_id")
		denomIn := q.Get("denom_in")
		amountIn := q.Get("amount_in")

		if poolIdStr == "" || denomIn == "" || amountIn == "" {
			writeDEXError(w, http.StatusBadRequest, "pool_id, denom_in, and amount_in are required")
			return
		}
		poolId, err := strconv.ParseUint(poolIdStr, 10, 64)
		if err != nil {
			writeDEXError(w, http.StatusBadRequest, "invalid pool_id")
			return
		}

		ctx, err := queryCtx()
		if err != nil {
			writeDEXError(w, http.StatusServiceUnavailable, "node not ready: "+err.Error())
			return
		}
		pool, err := k.GetPool(ctx, poolId)
		if err != nil {
			writeDEXError(w, http.StatusNotFound, fmt.Sprintf("pool %d not found", poolId))
			return
		}

		var reserveIn, reserveOut, denomOut string
		if denomIn == pool.DenomA {
			reserveIn, reserveOut, denomOut = pool.ReserveA, pool.ReserveB, pool.DenomB
		} else if denomIn == pool.DenomB {
			reserveIn, reserveOut, denomOut = pool.ReserveB, pool.ReserveA, pool.DenomA
		} else {
			writeDEXError(w, http.StatusBadRequest, fmt.Sprintf("denom %s not in pool", denomIn))
			return
		}

		amountOut, _, _, err := types.ComputeSwapOutput(reserveIn, reserveOut, amountIn)
		if err != nil {
			writeDEXError(w, http.StatusBadRequest, "compute swap: "+err.Error())
			return
		}

		writeDEXJSON(w, map[string]interface{}{
			"pool_id":    poolId,
			"denom_in":   denomIn,
			"amount_in":  amountIn,
			"denom_out":  denomOut,
			"amount_out": amountOut,
			"fee_pct":    "0.3",
		})
	}).Methods("GET")

	// GET /amitis/amitisdex/v1/positions/{address}
	router.HandleFunc("/amitis/amitisdex/v1/positions/{address}", func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		addr := vars["address"]

		ctx, err := queryCtx()
		if err != nil {
			writeDEXError(w, http.StatusServiceUnavailable, "node not ready: "+err.Error())
			return
		}
		positions, err := k.GetPositionsByAddress(ctx, addr)
		if err != nil {
			writeDEXError(w, http.StatusInternalServerError, err.Error())
			return
		}
		if positions == nil {
			positions = []*types.LiquidityPosition{}
		}
		writeDEXJSON(w, map[string]interface{}{"positions": positions})
	}).Methods("GET")
}
