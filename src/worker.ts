import mineflayer from 'mineflayer';
import { CONFIG } from './config';
import { scoped } from './logger';
import { sleep, waitFor } from './util';

export type WorkerEvents = {
  onStatus?: (w: Worker, status: string) => void;
};

/**
 * Один аккаунт = один Worker.
 * Живёт постоянно: коннектится, держится на сервере, по запросу сканирует анархии.
 */
export class Worker {
  readonly username: string;
  private bot: any = null;
  private log: ReturnType<typeof scoped>;

  /** готов принимать задания */
  ready = false;
  /** сейчас идёт скан */
  busy = false;
  /** текущая анархия (или 'hub') */
  current = 'hub';
  private stopped = false;
  private reconnectTimer: any = null;

  constructor(username: string, private events: WorkerEvents = {}) {
    this.username = username;
    this.log = scoped(username);
  }

  private status(s: string) {
    this.log.info(s);
    this.events.onStatus?.(this, s);
  }

  connect() {
    if (this.stopped) return;
    this.ready = false;
    this.status('подключение…');

    const bot = mineflayer.createBot({
      host: CONFIG.host,
      port: CONFIG.port,
      username: this.username,
      version: CONFIG.version,
      auth: 'offline',
      hideErrors: true,
    } as any);
    this.bot = bot;

    bot.on('error', (e: any) => this.log.error(`ошибка: ${e?.message ?? e}`));
    bot.on('kicked', (r: any) => this.log.warn(`кик: ${typeof r === 'string' ? r : JSON.stringify(r)}`));
    bot.on('end', () => {
      this.ready = false;
      this.busy = false;
      if (this.stopped) return;
      this.status(`отключён, переподключение через ${CONFIG.reconnectDelayMs / 1000}с`);
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = setTimeout(() => this.connect(), CONFIG.reconnectDelayMs);
    });

    bot.once('spawn', async () => {
      this.status(`зашёл, жду ${CONFIG.joinDelayMs / 1000}с`);
      await sleep(CONFIG.joinDelayMs);
      if (!this.bot) return;
      this.current = 'hub';
      this.ready = true;
      this.status('готов к поиску');
    });
  }

  stop() {
    this.stopped = true;
    clearTimeout(this.reconnectTimer);
    try {
      this.bot?.quit();
    } catch {}
  }

  private send(cmd: string) {
    this.bot?.chat(cmd.startsWith('/') ? cmd : `/${cmd}`);
  }

  /** Список ников онлайн на текущем сервере */
  private playerList(): string[] {
    const players = this.bot?.players ?? {};
    return Object.keys(players).filter((n) => n && n !== this.username);
  }

  /** Ждём, пока tab-лист «устаканится» */
  private async collectPlayers(): Promise<string[]> {
    const start = Date.now();
    let last = this.playerList().length;
    let lastChange = Date.now();
    while (Date.now() - start < CONFIG.playerListMaxMs) {
      await sleep(250);
      const n = this.playerList().length;
      if (n !== last) {
        last = n;
        lastChange = Date.now();
      } else if (Date.now() - lastChange >= CONFIG.playerListSettleMs && n > 0) {
        break;
      }
    }
    return this.playerList();
  }

  /**
   * Переход на анархию + сбор списка игроков.
   * Возвращает список ников (или null, если переход не удался).
   */
  async scanAnarchy(anarchy: string): Promise<string[] | null> {
    if (!this.bot) return null;
    this.send(`/${anarchy}`);
    const ok = await waitFor(this.bot, 'spawn', CONFIG.switchTimeoutMs);
    if (!ok) {
      this.log.warn(`не удалось перейти на ${anarchy}`);
      return null;
    }
    this.current = anarchy;
    const players = await this.collectPlayers();
    return players;
  }

  async toHub() {
    if (!this.bot) return;
    this.send('/hub');
    await waitFor(this.bot, 'spawn', CONFIG.switchTimeoutMs);
    this.current = 'hub';
  }
}
