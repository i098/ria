import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const [mode, ...extraArgs] = process.argv.slice(2);
if ((mode !== 'data' && mode !== 'samples') || extraArgs.length !== 0) {
  throw new Error('Usage: node extract.mjs <data|samples>');
}

const source = fileURLToPath(new URL('../../../ardia/', import.meta.url));
const commit = '551d3c13c1c9ede130724ea9f1056d4b586fbe55';
const blob = execFileSync('git', ['-C', source, 'show', `${commit}:packages/core/src/bangList.ts`]);
const { BANG_LIST, BANG_LIST_UNICODE } = await import(`data:text/javascript;base64,${blob.toString('base64')}`);
const lists = [BANG_LIST, BANG_LIST_UNICODE];

if (mode === 'data') {
  process.stdout.write(`${lists.join('\n')}\n`);
} else {
  for (const list of lists) {
    const rows = list.split('\n');
    const samples = Array.from({ length: 100 }, (_, i) => {
      const index = Math.floor(i * (rows.length - 1) / 99);
      return `${index}\t${rows[index]}`;
    });
    process.stdout.write(`${rows.length}\n${samples.join('\n')}\n`);
  }
}
