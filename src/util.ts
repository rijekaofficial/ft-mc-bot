export const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

export function waitFor(
  emitter: { once: (e: string, cb: (...a: any[]) => void) => void; removeListener: (e: string, cb: any) => void },
  event: string,
  timeoutMs: number,
): Promise<boolean> {
  return new Promise((resolve) => {
    let done = false;
    const onEvent = () => {
      if (done) return;
      done = true;
      clearTimeout(t);
      resolve(true);
    };
    const t = setTimeout(() => {
      if (done) return;
      done = true;
      emitter.removeListener(event, onEvent);
      resolve(false);
    }, timeoutMs);
    emitter.once(event, onEvent);
  });
}
