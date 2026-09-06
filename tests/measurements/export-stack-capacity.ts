/** Run: npx vite-node tests/measurements/export-stack-capacity.ts */
import { stackCapacityReport } from './stack-capacity.js';

process.stdout.write(
  `${JSON.stringify({ schema: 1, measurements: stackCapacityReport() }, null, 2)}\n`,
);
