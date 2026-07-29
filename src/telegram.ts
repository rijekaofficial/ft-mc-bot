import { scoped } from './logger.js';

const log = scoped('tg');

export type TgMessage = {
  chatId: number;
  from: string;
  text: string;
};

/**
 * Минимальный Telegram-клиент на long polling (без зависимостей).
 */
export class Telegram {
  private offset = 0;
  private stopped = false;
  private api: string;

  constructor(token: string, private allowedChats: string[] = []) {
    this.api = `https://api.telegram.org/bot${token}`;
  }

  private allowed(chatId: number) {
    return this.allowedChats.length === 0 || this.allowedChats.includes(String(chatId));
  }

  async call(method: string, body: Record<string, unknown>): Promise<any> {
    try {
      const res = await fetch(`${this.api}/${method}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body),
      });
      const json: any = await res.json();
      if (!json.ok) log.warn(`${method}: ${json.description}`);
      return json.result;
    } catch (e: any) {
      log.error(`${method}: ${e?.message ?? e}`);
      return null;
    }
  }

  send(chatId: number, text: string) {
    return this.call('sendMessage', {
      chat_id: chatId,
      text,
      parse_mode: 'HTML',
      disable_web_page_preview: true,
    });
  }

  editMessage(chatId: number, messageId: number, text: string) {
    return this.call('editMessageText', {
      chat_id: chatId,
      message_id: messageId,
      text,
      parse_mode: 'HTML',
    });
  }

  stop() {
    this.stopped = true;
  }

  async poll(handler: (m: TgMessage) => Promise<void> | void) {
    log.info('long polling запущен');
    while (!this.stopped) {
      const updates = await this.call('getUpdates', { offset: this.offset, timeout: 30 });
      if (!updates) {
        await new Promise((r) => setTimeout(r, 3000));
        continue;
      }
      for (const u of updates) {
        this.offset = u.update_id + 1;
        const msg = u.message ?? u.edited_message;
        if (!msg?.text) continue;
        if (!this.allowed(msg.chat.id)) {
          await this.send(msg.chat.id, `⛔️ Доступ запрещён. Ваш chat id: <code>${msg.chat.id}</code>`);
          continue;
        }
        try {
          await handler({
            chatId: msg.chat.id,
            from: msg.from?.username ?? msg.from?.first_name ?? 'unknown',
            text: String(msg.text).trim(),
          });
        } catch (e: any) {
          log.error(`handler: ${e?.message ?? e}`);
        }
      }
    }
  }
}
