// Кроссплатформенная сборка без .bin-шимов (важно для Termux).
// Находит tsc внутри пакета typescript и запускает его в текущем процессе.
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function findTsc() {
  // 1) обычный путь через резолв пакета
  try {
    const pkg = require.resolve('typescript/package.json', { paths: [root] });
    const dir = path.dirname(pkg);
    for (const rel of ['lib/tsc.js', 'bin/tsc']) {
      const p = path.join(dir, rel);
      if (existsSync(p)) return p;
    }
  } catch {}
  // 2) прямые пути на случай странного дерева зависимостей
  for (const rel of [
    'node_modules/typescript/lib/tsc.js',
    'node_modules/typescript/bin/tsc',
  ]) {
    const p = path.join(root, rel);
    if (existsSync(p)) return p;
  }
  return null;
}

const tsc = findTsc();

if (!tsc) {
  console.error(
    [
      '',
      '✗ Не найден компилятор TypeScript.',
      '',
      '  Скорее всего devDependencies не установились. Выполни:',
      '',
      '    npm install --include=dev',
      '  или:',
      '    npm install typescript@5 @types/node --save-dev',
      '',
    ].join('\n'),
  );
  process.exit(1);
}

process.argv = [process.argv[0], tsc, '-p', path.join(root, 'tsconfig.json'), ...process.argv.slice(2)];
await import(path.isAbsolute(tsc) ? new URL(`file://${tsc}`).href : tsc);
