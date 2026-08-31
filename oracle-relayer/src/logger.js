const winston = require('winston');

const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.errors({ stack: true }),
    winston.format.json()
  ),
  transports: [
    new winston.transports.Console({
      format: winston.format.combine(
        winston.format.colorize(),
        winston.format.printf(({ timestamp, level, message, ...meta }) => {
          const metaStr = Object.keys(meta).length
            ? '\n  ' + JSON.stringify(meta, null, 2).replace(/\n/g, '\n  ')
            : '';
          return `${timestamp} [${level}] ${message}${metaStr}`;
        })
      )
    }),
    new winston.transports.File({
      filename: 'relayer-error.log',
      level: 'error',
    }),
    new winston.transports.File({
      filename: 'relayer.log',
    }),
  ],
});

module.exports = logger;
