import { loadEnv } from './env.js';

loadEnv();

export const CONFIG = {
  host: process.env.MC_HOST ?? 'play.funtime.su',
  port: Number(process.env.MC_PORT ?? 25565),
  version: process.env.MC_VERSION ?? '1.16.5',

  // Пиратский сервер — оффлайн-авторизация по нику
  usernames: (process.env.MC_USERS ?? 'ir1skaa_ft_1,ir1skaa_ft_2,ir1skaa_ft_3')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean),

  // Пауза после захода на сервер перед началом работы
  joinDelayMs: Number(process.env.JOIN_DELAY_MS ?? 10_000),

  // Сколько ждать переключения сервера (/anXXX)
  switchTimeoutMs: Number(process.env.SWITCH_TIMEOUT_MS ?? 15_000),
  // Тишина в tab-листе, после которой считаем список игроков собранным
  playerListSettleMs: Number(process.env.PLAYERLIST_SETTLE_MS ?? 1_500),
  // Максимум ожидания сбора списка игроков после спавна
  playerListMaxMs: Number(process.env.PLAYERLIST_MAX_MS ?? 6_000),
  // Пауза между командами, чтобы не поймать антифлуд
  betweenAnarchyMs: Number(process.env.BETWEEN_ANARCHY_MS ?? 800),

  // Реконнект
  reconnectDelayMs: Number(process.env.RECONNECT_DELAY_MS ?? 10_000),

  telegramToken: process.env.TG_TOKEN ?? '8929651144:AAEPndWufv4HBF9kI_y_8wxzhaDnBrvFcPs',
  // Если задан — принимаем команды только от этих chat id (через запятую)
  telegramAllowedChats: (process.env.TG_CHATS ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean),
};
