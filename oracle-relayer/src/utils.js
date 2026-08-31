const axios = require('axios');
const logger = require('./logger');

/**
 * Resolve a dot-notation JSON path in an object.
 * e.g. resolveJsonPath({a: {b: 42}}, "a.b") => 42
 * Also supports array indexing: "data.items.0.value"
 */
function resolveJsonPath(obj, path) {
  if (!path) return obj;
  return path.split('.').reduce((current, key) => {
    if (current === null || current === undefined) return undefined;
    return current[key];
  }, obj);
}

/**
 * Scale a decimal number to micro units.
 * e.g. scaleToMicro(1000000.50, 6) => 1000000500000n
 */
function scaleToMicro(value, decimals = 6) {
  // Handle floating point precision issues by working with strings
  const str = value.toFixed(decimals);
  const [intPart, fracPart = ''] = str.split('.');
  const paddedFrac = fracPart.padEnd(decimals, '0').slice(0, decimals);
  return BigInt(intPart + paddedFrac);
}

/**
 * Send alert to configured webhook and/or email.
 */
async function sendAlert(jobId, severity, message) {
  const webhookUrl = process.env.ALERT_WEBHOOK_URL;

  if (!webhookUrl) {
    logger.warn('No ALERT_WEBHOOK_URL configured — alert not sent', { severity, message });
    return;
  }

  const payload = {
    text: `[Meridian Oracle Relayer] ${severity.toUpperCase()} — Job: ${jobId}\n${message}`,
    // Slack/Discord compatible
    attachments: [{
      color: severity === 'critical' ? '#ff0000' : severity === 'warning' ? '#ffaa00' : '#00aa00',
      fields: [
        { title: 'Job ID', value: jobId, short: true },
        { title: 'Severity', value: severity.toUpperCase(), short: true },
        { title: 'Message', value: message, short: false },
        { title: 'Time', value: new Date().toISOString(), short: true },
      ],
    }],
  };

  try {
    await axios.post(webhookUrl, payload, { timeout: 10000 });
    logger.info('Alert sent', { job_id: jobId, severity });
  } catch (e) {
    logger.error('Failed to send alert', { error: e.message });
  }
}

/**
 * Format micro amount as human-readable value.
 * e.g. formatMicro(1000000500000n, 6) => "1,000,000.50"
 */
function formatMicro(microAmount, decimals = 6) {
  const str = microAmount.toString().padStart(decimals + 1, '0');
  const intPart = str.slice(0, -decimals) || '0';
  const fracPart = str.slice(-decimals);
  const formatted = parseInt(intPart).toLocaleString();
  return `${formatted}.${fracPart}`;
}

module.exports = { resolveJsonPath, scaleToMicro, sendAlert, formatMicro };
