use teloxide::prelude::*;
use tokio::sync::mpsc;

use crate::config::TG_BOT_TOKEN;
use crate::state::SharedState;

/// Запускает Telegram бота
pub async fn start_tg_bot(shared: SharedState) {
    let bot = Bot::new(TG_BOT_TOKEN);

    // Создаём канал для отправки сообщений из MC бота в TG
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    shared.set_tg_sender(tx);

    // Сохраняем последний chat_id для уведомлений
    let chat_id_store = std::sync::Arc::new(parking_lot::Mutex::new(None::<ChatId>));

    // Спавним задачу для пересылки сообщений из MC в TG
    let bot_for_forward = bot.clone();
    let chat_id_for_forward = chat_id_store.clone();
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let chat_id = chat_id_for_forward.lock().clone();
            if let Some(chat_id) = chat_id {
                // Пробуем отправить как обычный текст (без Markdown)
                if let Err(e) = bot_for_forward.send_message(chat_id, &message).await {
                    eprintln!("⚠️ TG send error: {}", e);
                }
            } else {
                eprintln!("⚠️ TG: Нет chat_id, сообщение: {}", message);
            }
        }
    });

    println!("📱 Telegram бот запущен. Отправьте /start боту для начала работы.");

    // Обработчик команд
    let shared_for_handler = shared.clone();
    let chat_id_for_handler = chat_id_store.clone();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
            let shared = shared_for_handler.clone();
            let chat_id_store = chat_id_for_handler.clone();
            async move {
                // Сохраняем chat_id для уведомлений от MC ботов
                *chat_id_store.lock() = Some(msg.chat.id);

                if let Some(text) = msg.text() {
                    let text = text.trim();

                    if text == "/start" {
                        let welcome = "\
🤖 <b>FunTime Anarchy Finder Bot</b>\n\
\n\
Отправьте ник игрока для поиска по анархиям.\n\
\n\
<b>Команды:</b>\n\
/start — Показать это сообщение\n\
/status — Статус ботов\n\
/stop — Остановить поиск\n\
\n\
<i>Или просто отправьте ник игрока для поиска</i>";

                        let _ = bot
                            .send_message(msg.chat.id, welcome)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await;
                    } else if text == "/status" {
                        let ready = if shared.all_bots_ready() {
                            "✅ готовы"
                        } else {
                            "⏳ подключаются..."
                        };
                        let searching = if shared.is_search_active() {
                            if let Some(nick) = shared.get_target_nick() {
                                format!("🔍 ищем: {}", nick)
                            } else {
                                "🔍 поиск активен".to_string()
                            }
                        } else if shared.is_found() {
                            "✅ найден, ожидание".to_string()
                        } else {
                            "💤 ожидание".to_string()
                        };
                        let status = format!("🤖 Боты: {}\n🔎 Поиск: {}", ready, searching);
                        let _ = bot.send_message(msg.chat.id, status).await;
                    } else if text == "/stop" {
                        shared.stop_search();
                        let _ = bot
                            .send_message(msg.chat.id, "⏹ Поиск остановлен.")
                            .await;
                    } else if !text.starts_with('/') {
                        // Это ник для поиска
                        if !shared.all_bots_ready() {
                            let _ = bot
                                .send_message(
                                    msg.chat.id,
                                    "⏳ Боты ещё не подключились к серверу. Подождите...",
                                )
                                .await;
                            return Ok::<(), eyre::Report>(());
                        }

                        if shared.is_search_active() {
                            let _ = bot
                                .send_message(
                                    msg.chat.id,
                                    "⚠️ Поиск уже идёт! Отправьте /stop для остановки.",
                                )
                                .await;
                            return Ok::<(), eyre::Report>(());
                        }

                        let nick = text.to_string();
                        let total_anarchies = crate::config::generate_anarchy_commands().len();

                        let start_msg = format!(
                            "🔍 Начинаю поиск игрока <b>{}</b> по {} анархиям...\n\
                             3 бота параллельно проверяют серверы.\n\
                             Это займёт некоторое время.",
                            nick, total_anarchies
                        );
                        let _ = bot
                            .send_message(msg.chat.id, start_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await;

                        // Запускаем поиск
                        shared.start_search(nick);
                    }
                }

                Ok(())
            }
        }));

    Dispatcher::builder(bot, handler)
        .build()
        .dispatch()
        .await;
}
