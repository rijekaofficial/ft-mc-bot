import { ANARCHIES } from './anarchies.js';
import { CONFIG } from './config.js';
import { scoped } from './logger.js';
import { sleep } from './util.js';
import type { Worker } from './worker.js';

const log = scoped('search');

export type SearchResult =
  | { found: true; nick: string; anarchy: string; by: string; scanned: number; ms: number }
  | { found: false; nick: string; scanned: number; failed: string[]; ms: number; cancelled: boolean };

export type ProgressCb = (info: { scanned: number; total: number; anarchy: string; by: string }) => void;

/**
 * Параллельный поиск ника по всем анархиям.
 * Общая очередь — два бота никогда не идут на одну и ту же анку.
 */
export class SearchManager {
  private running = false;
  private cancelRequested = false;
  private currentNick: string | null = null;
  private scanned = 0;
  private failed: string[] = [];

  constructor(private workers: Worker[]) {}

  get isRunning() {
    return this.running;
  }
  get nick() {
    return this.currentNick;
  }
  get progress() {
    return { scanned: this.scanned, total: ANARCHIES.length };
  }

  cancel() {
    if (this.running) this.cancelRequested = true;
  }

  async search(rawNick: string, onProgress?: ProgressCb): Promise<SearchResult> {
    if (this.running) throw new Error('Поиск уже выполняется');

    const nick = rawNick.trim();
    const target = nick.toLowerCase();
    const started = Date.now();

    this.running = true;
    this.cancelRequested = false;
    this.currentNick = nick;
    this.scanned = 0;
    this.failed = [];

    const queue = [...ANARCHIES];
    type Hit = { anarchy: string; by: string };
    const hitBox: { value: Hit | null } = { value: null };

    const available = this.workers.filter((w) => w.ready);
    if (available.length === 0) {
      this.running = false;
      this.currentNick = null;
      return { found: false, nick, scanned: 0, failed: [], ms: 0, cancelled: true };
    }

    log.info(`старт поиска "${nick}" на ${ANARCHIES.length} анархиях, ботов: ${available.length}`);

    const runWorker = async (w: Worker) => {
      w.busy = true;
      try {
        while (!hitBox.value && !this.cancelRequested) {
          const anarchy = queue.shift();
          if (!anarchy) break;

          const players = await w.scanAnarchy(anarchy);
          this.scanned++;

          if (players === null) {
            this.failed.push(anarchy);
          } else {
            onProgress?.({ scanned: this.scanned, total: ANARCHIES.length, anarchy, by: w.username });
            const hit = players.find((p) => p.toLowerCase() === target);
            if (hit) {
              hitBox.value = { anarchy, by: w.username };
              log.info(`НАЙДЕН ${hit} на ${anarchy} (бот ${w.username})`);
              break;
            }
          }
          await sleep(CONFIG.betweenAnarchyMs);
        }
      } finally {
        w.busy = false;
      }
    };

    await Promise.all(available.map((w) => runWorker(w)));

    // Поиск завершён — только теперь возвращаемся в хаб
    await Promise.all(available.map((w) => w.toHub().catch(() => {})));

    const ms = Date.now() - started;
    const scanned = this.scanned;
    const failed = this.failed;
    const cancelled = this.cancelRequested;

    this.running = false;
    this.currentNick = null;

    const hit = hitBox.value;
    if (hit) {
      return { found: true, nick, anarchy: hit.anarchy, by: hit.by, scanned, ms };
    }
    return { found: false, nick, scanned, failed, ms, cancelled };
  }
}
