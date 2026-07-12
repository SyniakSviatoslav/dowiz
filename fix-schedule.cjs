const fs = require('fs');
const files = [
  'apps/api/src/server.ts',
  'apps/api/src/workers/anonymizer-retention.ts',
  'apps/api/src/workers/courier-cron.ts',
  'apps/api/src/workers/dwell-monitor.ts',
  'apps/api/src/workers/liveness-checker.ts',
  'apps/api/src/workers/settlement-cron.ts',
  'apps/api/src/workers/signal-raiser.ts',
  'apps/api/src/workers/backup/backup-verify-scheduled.ts',
  'apps/api/src/workers/backup/index.ts'
];
files.forEach(file => {
  const content = fs.readFileSync(file, 'utf8');
  const newContent = content.replace(/( +)await (this\.boss|queue\.boss)\.schedule\(['"]([^'"]+)['"]/g, (match, space, bossRef, queueName) => {
    return space + 'await ' + bossRef + '.createQueue(\'' + queueName + '\');\n' + match;
  });
  if (content !== newContent) {
    fs.writeFileSync(file, newContent, 'utf8');
    console.log('Updated ' + file);
  }
});
