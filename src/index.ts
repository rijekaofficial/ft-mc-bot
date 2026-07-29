import { ANARCHIES } from './anarchies';
import { CONFIG } from './config';
import { scoped } from './logger';
import { SearchManager } from './search';
import { Telegram } from './telegram';
import { Worker } from './worker';

const log = scoped('main');

const workers = CONFIG.usernames.map((u) => new Worker(u));
const search = new SearchManager(workers);
const tg = new Telegram(CONFIG.telegramToken, CONFIG.telegramAllowedChats);

for (const w of workers) w.connect();

const HELP = [
  '<b>FunTime anarchy finder</b>',
  '',
  'Просто пришли ник — начнётся поиск.',
  '',
  '<b>Команды</b>',
  '/find &lt;ник&gt; — искать игрока',
  '/status — состояние ботов',
  '/stop — отменить текущий поиск',
  '/list — список анархий',
  '/help — эта справка',
].join('\n');

function statusText() {
  const lines = workers.map((w) => {
    const state = !w.ready ? '🔴 оффлайн' : w.busy ? '🟡 сканирует' : '🟢 готов';
    return `• <code>${w.username}</code> — ${state} (${w.current})`;
  });
  lines.push('');
  lines.push(
    search.isRunning
      ? `🔎 Идёт поиск <b>${search.nick}</b>: ${search.progress.scanned}/${search.progress.total}`
      : '💤 Поиск не выполняется',
  );
  lines.push(`Анархий в списке: ${ANARCHIES.length}`);
  return lines.join('\n');
}

function fmtMs(ms: number) {
  const s = Math.round(ms / 1000);
  return s < 60 ? `${s}с` : `${Math.floor(s / 60)}м ${s % 60}с`;
}

async function startSearch(chatId: number, nick: string) {
  if (search.isRunning) {
    await tg.send(chatId, `⚠️ Уже ищу <b>${search.nick}</b> (${search.progress.scanned}/${search.progress.total}). /stop чтобы отменить.`);
    return;
  }
  if (!/^[A-Za-z0-9_]{2,16}$/.test(nick)) {
    await tg.send(chatId, '⚠️ Некорректный ник.');
    return;
  }
  const online = workers.filter((w) => w.ready).length;
  if (online === 0) {
    await tg.send(chatId, '⛔️ Ни один бот не в сети, подожди подключения.');
    return;
  }

  const msg = await tg.send(chatId, `🔎 Ищу <b>${nick}</b>…\nБотов: ${online} • Анархий: ${ANARCHIES.length}`);
  const msgId = msg?.message_id;
  let lastEdit = 0;

  const result = await search.search(nick, ({ scanned, total, anarchy, by }) => {
    const now = Date.now();
    if (!msgId || now - lastEdit < 4000) return;
    lastEdit = now;
    void tg.editMessage(
      chatId,
      msgId,
      `🔎 Ищу <b>${nick}</b>…\nПрогресс: ${scanned}/${total}\nПоследняя: <code>/${anarchy}</code> (${by})`,
    );
  });

  if (result.found) {
    await tg.send(
      chatId,
      [
        `✅ Игрок <b>${result.nick}</b> найден!`,
        `Анархия: <b>/${result.anarchy}</b>`,
        `Нашёл: <code>${result.by}</code>`,
        `Проверено анархий: ${result.scanned}/${ANARCHIES.length} за ${fmtMs(result.ms)}`,
        '',
        'Боты вернулись в /hub и ждут следующий ник.',
      ].join('\n'),
    );
  } else if (result.cancelled) {
    await tg.send(chatId, `🛑 Поиск <b>${result.nick}</b> отменён (проверено ${result.scanned}).`);
  } else {
    await tg.send(
      chatId,
      [
        `❌ Игрок <b>${result.nick}</b> не найден.`,
        `Проверено: ${result.scanned}/${ANARCHIES.length} за ${fmtMs(result.ms)}`,
        result.failed.length ? `Не удалось зайти: ${result.failed.map((a) => '/' + a).join(', ')}` : '',
      ]
        .filter(Boolean)
        .join('\n'),
    );
  }
}

await tg.call('setMyCommands', {
  commands: [
    { command: 'find', description: 'Искать игрока по нику' },
    { command: 'status', description: 'Состояние ботов' },
    { command: 'stop', description: 'Отменить поиск' },
    { command: 'list', description: 'Список анархий' },
    { command: 'help', description: 'Справка' },
  ],
});

void tg.poll(async ({ chatId, text }) => {
  const [cmdRaw, ...rest] = text.split(/\s+/);
  const cmd = cmdRaw.toLowerCase().replace(/@.*$/, '');

  switch (cmd) {
    case '/start':
    case '/help':
      await tg.send(chatId, HELP);
      return;
    case '/status':
      await tg.send(chatId, statusText());
      return;
    case '/stop':
      if (!search.isRunning) await tg.send(chatId, 'Сейчас ничего не ищется.');
      else {
        search.cancel();
        await tg.send(chatId, '🛑 Останавливаю поиск…');
      }
      return;
    case '/list':
      await tg.send(chatId, `Всего ${ANARCHIES.length} анархий:\n<code>${ANARCHIES.map((a) => '/' + a).join(' ')}</code>`);
      return;
    case '/find':
      if (!rest[0]) await tg.send(chatId, 'Использование: /find &lt;ник&gt;');
      else await startSearch(chatId, rest[0]);
      return;
    default:
      if (cmd.startsWith('/')) {
        await tg.send(chatId, 'Неизвестная команда. /help');
        return;
      }
      await startSearch(chatId, cmdRaw);
  }
});

log.info(`боты: ${CONFIG.usernames.join(', ')} • анархий: ${ANARCHIES.length}`);

for (const sig of ['SIGINT', 'SIGTERM'] as const) {
  process.on(sig, () => {
    log.info('выключение…');
    tg.stop();
    workers.forEach((w) => w.stop());
    process.exit(0);
  });
}
