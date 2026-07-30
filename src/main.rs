//! FunTime MC Bot — параллельный поиск игроков по анархиям
//!
//! ## Как это работает
//!
//! 1. 3 Minecraft бота подключаются к `play.funtime.su` (оффлайн-аккаунты)
//! 2. Отклоняют ресурс-пак и ждут 10 секунд
//! 3. Telegram бот принимает ник игрока для поиска
//! 4. 3 бота параллельно проверяют анархии (/an101–/an904),
//!    каждый берёт следующую анархию из общей очереди
//! 5. После написания /anXXX бот читает список игроков (TabList)
//!    и проверяет, есть ли там искомый ник
//! 6. При нахождении — уведомление в Telegram и возврат на /hub

pub mod config;
pub mod mc_bot;
pub mod state;
pub mod tg_bot;

use state::SharedState;

fn main() {
    println!("🚀 FunTime MC Bot запускается...");
    println!(
        "   Сервер: {}",
        config::MC_SERVER
    );
    println!(
        "   Боты: {}",
        config::BOT_NICKNAMES.join(", ")
    );
    println!(
        "   Анархий для проверки: {}",
        config::generate_anarchy_commands().len()
    );

    // Создаём общий стейт
    let shared = SharedState::new(config::BOT_NICKNAMES.len());

    // Запускаем MC ботов в отдельном потоке
    // (azalea требует current_thread runtime внутри)
    let shared_mc = shared.clone();
    let mc_thread = std::thread::Builder::new()
        .name("mc-bots".to_string())
        .stack_size(8 * 1024 * 1024) // 8 MB стек
        .spawn(move || {
            mc_bot::start_mc_bots(shared_mc);
        })
        .expect("Failed to spawn MC bots thread");

    // Запускаем Telegram бот в основном tokio runtime
    let shared_tg = shared.clone();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime for TG bot");

    rt.block_on(async move {
        tg_bot::start_tg_bot(shared_tg).await;
    });

    // Ждём MC поток (обычно не доходит сюда)
    let _ = mc_thread.join();
}
