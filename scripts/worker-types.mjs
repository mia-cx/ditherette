import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

execFileSync('pnpm', ['exec', 'wrangler', 'types'], { stdio: 'inherit' });

const file = new URL('../worker-configuration.d.ts', import.meta.url);
const types = readFileSync(file, 'utf8');
// SvelteKit owns the Worker entrypoint. Importing its generated JS makes checkJs inspect build output.
// Retain binding/runtime types; remove this workaround when workers-sdk#14181 is fixed.
writeFileSync(file, types.replace(/\n\tinterface GlobalProps \{[\s\S]*?\n\t\}/, ''));
