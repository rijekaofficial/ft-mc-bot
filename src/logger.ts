type Level = 'info' | 'warn' | 'error';

function ts() {
  return new Date().toISOString().slice(11, 19);
}

export function log(scope: string, msg: string, level: Level = 'info') {
  const tag = level === 'error' ? '!!' : level === 'warn' ? ' ?' : ' ·';
  console.log(`[${ts()}]${tag} [${scope}] ${msg}`);
}

export const scoped = (scope: string) => ({
  info: (m: string) => log(scope, m, 'info'),
  warn: (m: string) => log(scope, m, 'warn'),
  error: (m: string) => log(scope, m, 'error'),
});
