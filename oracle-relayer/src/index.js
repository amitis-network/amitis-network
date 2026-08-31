/**
 * Meridian Oracle Relayer
 *
 * Fetches reserve data from external sources and submits
 * SubmitReserve transactions to PoR Aggregator contracts on Amitis Network.
 *
 * Supports:
 *   - HTTP API endpoints (fiat bank APIs, custodian APIs)
 *   - On-chain balance queries (via REST)
 *   - Multiple aggregators running on independent schedules
 *   - Retry logic with exponential backoff
 *   - SQLite job history and alerting
 */

require('dotenv').config();
const cron = require('node-cron');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { GasPrice } = require('@cosmjs/stargate');
const axios = require('axios');
const Database = require('better-sqlite3');
const logger = require('./logger');
const { resolveJsonPath, scaleToMicro, sendAlert } = require('./utils');

// ── Config ─────────────────────────────────────────────────────────────────────

const CHAIN_ID      = process.env.CHAIN_ID || 'amitis-network';
const RPC_ENDPOINT  = process.env.RPC_ENDPOINT || 'https://rpc.amitis.network:443';
const REST_ENDPOINT = process.env.REST_ENDPOINT || 'https://rest.amitis.network';
const MNEMONIC      = process.env.ORACLE_MNEMONIC;
const GAS_PRICE     = process.env.GAS_PRICE || '0.025uamts';
const GAS_LIMIT     = parseInt(process.env.GAS_LIMIT || '200000');
const DB_PATH       = process.env.DB_PATH || './relayer.db';

let JOBS = [];
try {
  JOBS = JSON.parse(process.env.AGGREGATOR_JOBS || '[]');
} catch (e) {
  logger.error('Failed to parse AGGREGATOR_JOBS env var', { error: e.message });
  process.exit(1);
}

// ── Database ──────────────────────────────────────────────────────────────────

const db = new Database(DB_PATH);

db.exec(`
  CREATE TABLE IF NOT EXISTS job_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    aggregator_addr TEXT NOT NULL,
    status TEXT NOT NULL,
    raw_value TEXT,
    scaled_value TEXT,
    tx_hash TEXT,
    error TEXT,
    duration_ms INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    sent_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE INDEX IF NOT EXISTS idx_job_runs_job_id ON job_runs(job_id);
  CREATE INDEX IF NOT EXISTS idx_job_runs_created_at ON job_runs(created_at);
`);

const insertRun = db.prepare(`
  INSERT INTO job_runs (job_id, aggregator_addr, status, raw_value, scaled_value, tx_hash, error, duration_ms)
  VALUES (@job_id, @aggregator_addr, @status, @raw_value, @scaled_value, @tx_hash, @error, @duration_ms)
`);

// ── Cosmos client ──────────────────────────────────────────────────────────────

let cosmClient = null;
let oracleAddress = null;

async function getClient() {
  if (cosmClient) return { client: cosmClient, address: oracleAddress };

  if (!MNEMONIC) {
    throw new Error('ORACLE_MNEMONIC environment variable not set');
  }

  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(MNEMONIC, {
    prefix: 'amitis',
  });

  const accounts = await wallet.getAccounts();
  oracleAddress = accounts[0].address;

  cosmClient = await SigningCosmWasmClient.connectWithSigner(
    RPC_ENDPOINT,
    wallet,
    { gasPrice: GasPrice.fromString(GAS_PRICE) }
  );

  logger.info('Cosmos client connected', {
    address: oracleAddress,
    rpc: RPC_ENDPOINT,
  });

  return { client: cosmClient, address: oracleAddress };
}

// ── Reserve fetchers ──────────────────────────────────────────────────────────

/**
 * Fetch reserve from an HTTP API endpoint.
 * Returns the raw numeric value.
 */
async function fetchHttpApi(config) {
  const response = await axios({
    method: config.method || 'GET',
    url: config.url,
    headers: config.headers || {},
    timeout: 30000,
  });

  const raw = resolveJsonPath(response.data, config.json_path);
  if (raw === undefined || raw === null) {
    throw new Error(`JSON path '${config.json_path}' not found in response`);
  }

  const num = parseFloat(raw);
  if (isNaN(num) || num < 0) {
    throw new Error(`Invalid reserve value: ${raw}`);
  }

  return { raw: raw.toString(), value: num };
}

/**
 * Fetch on-chain CW20 balance via REST.
 * For USDC/USDT reserves held in a known wallet.
 */
async function fetchOnchainBalance(config) {
  // config.token_contract: CW20 contract address
  // config.holder_address: wallet holding the reserves
  const query = Buffer.from(JSON.stringify({
    balance: { address: config.holder_address }
  })).toString('base64');

  const url = `${REST_ENDPOINT}/cosmwasm/wasm/v1/contract/${config.token_contract}/smart/${query}`;
  const response = await axios.get(url, { timeout: 15000 });

  const balance = response.data?.data?.balance;
  if (!balance) {
    throw new Error('Could not read on-chain balance');
  }

  // CW20 balances are already in micro units
  const microValue = BigInt(balance);
  return {
    raw: balance,
    value: Number(microValue),
    alreadyMicro: true,
  };
}

/**
 * Fetch reserve value for a job based on its type.
 */
async function fetchReserveValue(job) {
  switch (job.type) {
    case 'http_api':
      return fetchHttpApi(job.config);
    case 'onchain_cw20':
      return fetchOnchainBalance(job.config);
    default:
      throw new Error(`Unknown job type: ${job.type}`);
  }
}

// ── Submit to aggregator ──────────────────────────────────────────────────────

async function submitToAggregator(aggregatorAddr, microAmount, reference) {
  const { client, address } = await getClient();

  const msg = {
    submit_reserve: {
      amount: microAmount.toString(),
      reference: reference || null,
    },
  };

  const result = await client.execute(
    address,
    aggregatorAddr,
    msg,
    'auto',
    `Meridian PoR relay — ${reference || 'scheduled'}`,
    [],
  );

  if (result.code !== 0) {
    throw new Error(`Transaction failed: ${result.rawLog}`);
  }

  return result.transactionHash;
}

// ── Job runner ────────────────────────────────────────────────────────────────

async function runJob(job, isManual = false) {
  const startTime = Date.now();
  logger.info('Running job', { job_id: job.id, label: job.label, manual: isManual });

  try {
    // Fetch reserve value
    const { raw, value, alreadyMicro } = await fetchReserveValue(job);
    logger.info('Fetched reserve value', { job_id: job.id, raw, value });

    // Scale to micro units (6 decimals) unless already in micro
    const microAmount = alreadyMicro
      ? BigInt(Math.round(value))
      : scaleToMicro(value, job.config.decimals || 6);

    logger.info('Scaled value', {
      job_id: job.id,
      raw_value: value,
      micro_amount: microAmount.toString(),
    });

    // Validate — basic sanity checks
    if (microAmount <= 0n) {
      throw new Error(`Reserve value is zero or negative: ${microAmount}`);
    }

    // Check against last known value — alert if >10% swing
    const lastRun = db.prepare(
      'SELECT scaled_value FROM job_runs WHERE job_id = ? AND status = ? ORDER BY created_at DESC LIMIT 1'
    ).get(job.id, 'success');

    if (lastRun) {
      const lastValue = BigInt(lastRun.scaled_value);
      if (lastValue > 0n) {
        const delta = Number(microAmount - lastValue) / Number(lastValue);
        if (Math.abs(delta) > 0.10) {
          const direction = delta > 0 ? 'UP' : 'DOWN';
          const pct = (Math.abs(delta) * 100).toFixed(2);
          logger.warn('Reserve value changed significantly', {
            job_id: job.id,
            direction,
            percent: pct,
            previous: lastValue.toString(),
            current: microAmount.toString(),
          });
          await sendAlert(job.id, 'warning',
            `Reserve ${direction} ${pct}% for ${job.label}: ${lastValue} → ${microAmount}`
          );
        }
      }
    }

    // Submit to aggregator
    const reference = `relayer-${job.id}-${Date.now()}`;
    const txHash = await submitToAggregator(job.aggregator_addr, microAmount, reference);
    const duration = Date.now() - startTime;

    logger.info('Submitted to aggregator', {
      job_id: job.id,
      tx_hash: txHash,
      duration_ms: duration,
    });

    insertRun.run({
      job_id: job.id,
      aggregator_addr: job.aggregator_addr,
      status: 'success',
      raw_value: raw?.toString(),
      scaled_value: microAmount.toString(),
      tx_hash: txHash,
      error: null,
      duration_ms: duration,
    });

    return { success: true, txHash, microAmount };

  } catch (error) {
    const duration = Date.now() - startTime;
    logger.error('Job failed', {
      job_id: job.id,
      error: error.message,
      duration_ms: duration,
    });

    insertRun.run({
      job_id: job.id,
      aggregator_addr: job.aggregator_addr,
      status: 'error',
      raw_value: null,
      scaled_value: null,
      tx_hash: null,
      error: error.message,
      duration_ms: duration,
    });

    // Alert on consecutive failures
    const recentFailures = db.prepare(
      'SELECT COUNT(*) as count FROM job_runs WHERE job_id = ? AND status = ? AND created_at > datetime("now", "-1 hour")'
    ).get(job.id, 'error');

    if (recentFailures.count >= 3) {
      await sendAlert(job.id, 'critical',
        `Job ${job.id} has failed ${recentFailures.count} times in the last hour: ${error.message}`
      );
    }

    return { success: false, error: error.message };
  }
}

// ── Retry wrapper ─────────────────────────────────────────────────────────────

async function runJobWithRetry(job, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    const result = await runJob(job);
    if (result.success) return result;

    if (attempt < maxRetries) {
      const delay = Math.pow(2, attempt) * 5000; // 10s, 20s, 40s
      logger.info(`Retrying job in ${delay}ms`, { job_id: job.id, attempt });
      await new Promise(r => setTimeout(r, delay));
    }
  }
  logger.error('Job exhausted all retries', { job_id: job.id });
}

// ── HTTP server for manual triggers and status ────────────────────────────────

function startHttpServer() {
  const http = require('http');
  const PORT = process.env.PORT || 3100;

  const server = http.createServer(async (req, res) => {
    res.setHeader('Content-Type', 'application/json');

    // GET /status — relayer health and last run info per job
    if (req.method === 'GET' && req.url === '/status') {
      const status = JOBS.map(job => {
        const lastRun = db.prepare(
          'SELECT * FROM job_runs WHERE job_id = ? ORDER BY created_at DESC LIMIT 1'
        ).get(job.id);
        const successCount = db.prepare(
          'SELECT COUNT(*) as c FROM job_runs WHERE job_id = ? AND status = ?'
        ).get(job.id, 'success').c;
        const errorCount = db.prepare(
          'SELECT COUNT(*) as c FROM job_runs WHERE job_id = ? AND status = ?'
        ).get(job.id, 'error').c;
        return {
          id: job.id,
          label: job.label,
          aggregator_addr: job.aggregator_addr,
          cron_schedule: job.cron_schedule,
          last_run: lastRun || null,
          success_count: successCount,
          error_count: errorCount,
        };
      });

      res.writeHead(200);
      res.end(JSON.stringify({
        relayer: 'meridian-oracle-relayer',
        oracle_address: oracleAddress,
        chain_id: CHAIN_ID,
        jobs: status,
      }, null, 2));
      return;
    }

    // POST /run/:jobId — manual trigger
    if (req.method === 'POST' && req.url.startsWith('/run/')) {
      const jobId = req.url.replace('/run/', '').split('?')[0];
      const job = JOBS.find(j => j.id === jobId);
      if (!job) {
        res.writeHead(404);
        res.end(JSON.stringify({ error: `Job '${jobId}' not found` }));
        return;
      }
      const result = await runJobWithRetry(job);
      res.writeHead(result?.success ? 200 : 500);
      res.end(JSON.stringify(result));
      return;
    }

    // GET /history/:jobId — last 20 runs for a job
    if (req.method === 'GET' && req.url.startsWith('/history/')) {
      const jobId = req.url.replace('/history/', '').split('?')[0];
      const runs = db.prepare(
        'SELECT * FROM job_runs WHERE job_id = ? ORDER BY created_at DESC LIMIT 20'
      ).all(jobId);
      res.writeHead(200);
      res.end(JSON.stringify(runs));
      return;
    }

    res.writeHead(404);
    res.end(JSON.stringify({ error: 'Not found' }));
  });

  server.listen(PORT, () => {
    logger.info(`Relayer HTTP server listening on port ${PORT}`);
  });
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  logger.info('Meridian Oracle Relayer starting', {
    chain_id: CHAIN_ID,
    jobs: JOBS.length,
  });

  if (JOBS.length === 0) {
    logger.error('No jobs configured — set AGGREGATOR_JOBS in .env');
    process.exit(1);
  }

  // Initialize client
  try {
    await getClient();
  } catch (e) {
    logger.error('Failed to connect to chain', { error: e.message });
    process.exit(1);
  }

  // Schedule all jobs
  for (const job of JOBS) {
    logger.info('Scheduling job', {
      id: job.id,
      schedule: job.cron_schedule,
      aggregator: job.aggregator_addr,
    });

    cron.schedule(job.cron_schedule, async () => {
      await runJobWithRetry(job);
    });

    // Run immediately on startup
    logger.info('Running initial job execution', { id: job.id });
    await runJobWithRetry(job);
  }

  // Start HTTP management server
  startHttpServer();

  logger.info('Oracle relayer running. All jobs scheduled.');
}

main().catch(e => {
  logger.error('Fatal error', { error: e.message, stack: e.stack });
  process.exit(1);
});
