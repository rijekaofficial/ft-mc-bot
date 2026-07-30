/// Конфигурация бота

/// Адрес Minecraft сервера
pub const MC_SERVER: &str = "play.funtime.su";

/// Никнеймы ботов (3 аккаунта)
pub const BOT_NICKNAMES: [&str; 3] = ["ir1skaa_ft_1", "ir1skaa_ft_2", "ir1skaa_ft_3"];

/// Токен Telegram бота
pub const TG_BOT_TOKEN: &str = "8929651144:AAEPndWufv4HBF9kI_y_8wxzhaDnBrvFcPs";

/// Задержка после подключения перед началом работы (секунды)
pub const INITIAL_DELAY_SECS: u64 = 10;

/// Задержка между переключениями анархий (секунды) — ждём загрузку таблиста
pub const ANARCHY_SWITCH_DELAY_SECS: u64 = 3;

/// Генерирует полный список команд анархий
pub fn generate_anarchy_commands() -> Vec<String> {
    let mut commands = Vec::new();

    // /an101 - /an114
    for i in 101..=114 {
        commands.push(format!("/an{}", i));
    }
    // /an201 - /an228
    for i in 201..=228 {
        commands.push(format!("/an{}", i));
    }
    // /an301 - /an325
    for i in 301..=325 {
        commands.push(format!("/an{}", i));
    }
    // /an501 - /an516
    for i in 501..=516 {
        commands.push(format!("/an{}", i));
    }
    // /an901 - /an904
    for i in 901..=904 {
        commands.push(format!("/an{}", i));
    }

    commands
}
