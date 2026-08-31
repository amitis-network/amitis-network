package amitisbridge

import (
	"encoding/json"
	"net/http"

	"github.com/gorilla/mux"
	sdk "github.com/cosmos/cosmos-sdk/types"

	"github.com/amitis-network/amitis-network/x/amitisbridge/keeper"
	"github.com/amitis-network/amitis-network/x/amitisbridge/types"
)

func RegisterAPIRoutes(router *mux.Router, k keeper.Keeper, queryCtx func() (sdk.Context, error)) {

	// GET /amitis/bridge/v1/params
	router.HandleFunc("/amitis/bridge/v1/params", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		writeJSON(w, map[string]interface{}{"params": k.GetParams(ctx)})
	}).Methods("GET", "OPTIONS")

	// GET /amitis/bridge/v1/stats
	router.HandleFunc("/amitis/bridge/v1/stats", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		stats, err := k.GetStats(ctx)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		writeJSON(w, stats)
	}).Methods("GET", "OPTIONS")

	// GET /amitis/bridge/v1/inbound
	router.HandleFunc("/amitis/bridge/v1/inbound", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		reqs, err := k.GetAllInboundRequests(ctx)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		// Optional status filter
		status := r.URL.Query().Get("status")
		if status != "" {
			var filtered []types.InboundRequest
			for _, req := range reqs {
				if req.Status == status {
					filtered = append(filtered, req)
				}
			}
			reqs = filtered
		}
		if reqs == nil {
			reqs = []types.InboundRequest{}
		}
		writeJSON(w, map[string]interface{}{"requests": reqs, "count": len(reqs)})
	}).Methods("GET", "OPTIONS")

	// GET /amitis/bridge/v1/inbound/{id}
	router.HandleFunc("/amitis/bridge/v1/inbound/{id}", func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		req, err := k.GetInboundRequest(ctx, vars["id"])
		if err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}
		attestation, _ := k.GetAttestation(ctx, vars["id"])
		writeJSON(w, map[string]interface{}{
			"request":      req,
			"attestations": attestation,
		})
	}).Methods("GET", "OPTIONS")

	// GET /amitis/bridge/v1/outbound
	router.HandleFunc("/amitis/bridge/v1/outbound", func(w http.ResponseWriter, r *http.Request) {
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		reqs, err := k.GetAllOutboundRequests(ctx)
		if err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		status := r.URL.Query().Get("status")
		if status != "" {
			var filtered []types.OutboundRequest
			for _, req := range reqs {
				if req.Status == status {
					filtered = append(filtered, req)
				}
			}
			reqs = filtered
		}
		if reqs == nil {
			reqs = []types.OutboundRequest{}
		}
		writeJSON(w, map[string]interface{}{"requests": reqs, "count": len(reqs)})
	}).Methods("GET", "OPTIONS")

	// GET /amitis/bridge/v1/outbound/{id}
	router.HandleFunc("/amitis/bridge/v1/outbound/{id}", func(w http.ResponseWriter, r *http.Request) {
		vars := mux.Vars(r)
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		req, err := k.GetOutboundRequest(ctx, vars["id"])
		if err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}
		writeJSON(w, req)
	}).Methods("GET", "OPTIONS")

	// POST /amitis/bridge/v1/attest
	// Body: MsgSubmitAttestation JSON
	router.HandleFunc("/amitis/bridge/v1/attest", func(w http.ResponseWriter, r *http.Request) {
		var msg types.MsgSubmitAttestation
		if err := json.NewDecoder(r.Body).Decode(&msg); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body: "+err.Error())
			return
		}
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		msgSrv := keeper.NewMsgServer(k)
		resp, err := msgSrv.SubmitAttestation(ctx, &msg)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}
		writeJSON(w, resp)
	}).Methods("POST", "OPTIONS")

	// POST /amitis/bridge/v1/withdraw
	// Body: MsgInitiateWithdrawal JSON
	router.HandleFunc("/amitis/bridge/v1/withdraw", func(w http.ResponseWriter, r *http.Request) {
		var msg types.MsgInitiateWithdrawal
		if err := json.NewDecoder(r.Body).Decode(&msg); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body: "+err.Error())
			return
		}
		ctx, err := queryCtx()
		if err != nil {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		msgSrv := keeper.NewMsgServer(k)
		resp, err := msgSrv.InitiateWithdrawal(ctx, &msg)
		if err != nil {
			writeError(w, http.StatusBadRequest, err.Error())
			return
		}
		writeJSON(w, resp)
	}).Methods("POST", "OPTIONS")

	// OPTIONS handler for CORS preflight
	router.HandleFunc("/amitis/bridge/v1/{path:.*}", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		w.WriteHeader(http.StatusNoContent)
	}).Methods("OPTIONS")
}

func writeJSON(w http.ResponseWriter, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, code int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{"error": msg})
}
