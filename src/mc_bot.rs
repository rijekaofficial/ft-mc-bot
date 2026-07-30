use std::time::Duration;

use azalea::ecs::prelude::*;
use azalea::prelude::*;
use azalea::swarm::prelude::*;

use crate::config::*;
use crate::state::SharedState;
use azalea::{Event, InConfigState};

/// Состояние отдельного бота в swarm
#[derive(Component, Clone)]
pub struct BotState {
    pub shared: SharedState,
    pub bot_name: String,
    /// Флаг: поисковая задача уже запущена
    pub search_task_spawned: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Состояние swarm (общее для всех ботов)
#[derive(Resource, Default, Clone)]
pub struct SwarmState {
    pub shared: SharedState,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            shared: SharedState::new(3),
            bot_name: String::new(),
            search_task_spawned: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// Запускает MC ботов (блокирует текущий поток — запускать в std::thread)
pub fn start_mc_bots(shared: SharedState) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime for MC bots");

    rt.block_on(async move {
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async move {
                let swarm_state = SwarmState {
                    shared: shared.clone(),
                };

                let mut builder = SwarmBuilder::new()
                    .set_handler(handle)
                    .set_swarm_handler(swarm_handle);

                // Добавляем плагин для отклонения ресурс-паков
                builder = builder.add_plugins(DeclineResourcePacksPlugin);

                for name in BOT_NICKNAMES {
                    let account = Account::offline(name);
                    let state = BotState {
                        shared: shared.clone(),
                        bot_name: name.to_string(),
                        search_task_spawned: std::sync::Arc::new(
                            std::sync::atomic::AtomicBool::new(false),
                        ),
                    };
                    builder = builder.add_account_with_state(account, state);
                }

                builder
                    .join_delay(Duration::from_secs(3))
                    .set_swarm_state(swarm_state)
                    .start(MC_SERVER)
                    .await;
            })
            .await;
    });
}

/// Handler для каждого бота
async fn handle(bot: Client, event: Event, state: BotState) -> eyre::Result<()> {
    match event {
        Event::Init => {
            println!(
                "[{}] Подключился к серверу, жду {} сек...",
                state.bot_name, INITIAL_DELAY_SECS
            );

            // Спавним async задачу для этого бота (только один раз)
            // Используем spawn_local т.к. azalea работает с LocalSet
            let should_spawn = !state
                .search_task_spawned
                .swap(true, std::sync::atomic::Ordering::SeqCst);

            if should_spawn {
                let bot = bot.clone();
                let shared = state.shared.clone();
                let bot_name = state.bot_name.clone();

                tokio::task::spawn_local(async move {
                    // Ждём начальную задержку
                    tokio::time::sleep(Duration::from_secs(INITIAL_DELAY_SECS)).await;

                    let ready = shared.increment_ready();
                    shared.send_to_tg(format!(
                        "🤖 {} подключился ({}/{})",
                        bot_name,
                        ready,
                        BOT_NICKNAMES.len()
                    ));

                    if shared.all_bots_ready() {
                        shared.send_to_tg(
                            "✅ Все боты подключены и готовы! Отправьте ник для поиска."
                                .to_string(),
                        );
                    }

                    // Основной цикл поиска
                    loop {
                        tokio::time::sleep(Duration::from_millis(500)).await;

                        // Если поиск не активен или уже нашли — просто ждём
                        if !shared.is_search_active() || shared.is_found() {
                            continue;
                        }

                        let Some(target) = shared.get_target_nick() else {
                            continue;
                        };

                        // Берём следующую анархию из очереди
                        let Some(anarchy_cmd) = shared.take_next_anarchy() else {
                            // Очередь пуста — этому боту больше нечего проверять
                            continue;
                        };

                        // Двойная проверка — вдруг другой бот уже нашёл
                        if shared.is_found() {
                            break;
                        }

                        // Отправляем команду перехода на анархию
                        println!("[{}] → {}", bot_name, anarchy_cmd);
                        bot.chat(&anarchy_cmd);

                        // Ждём переключения сервера и загрузки таблиста
                        tokio::time::sleep(Duration::from_secs(ANARCHY_SWITCH_DELAY_SECS)).await;

                        if shared.is_found() {
                            break;
                        }

                        // Проверяем таблист (список игроков)
                        let found = {
                            let ecs = bot.ecs.read();
                            check_tablist_for_player(&ecs, bot.entity, &target)
                        };

                        if found {
                            let msg = format!(
                                "🎯 {} нашёл игрока {} на {}!",
                                bot_name, target, anarchy_cmd
                            );
                            println!("{}", msg);
                            shared.send_to_tg(msg);
                            shared.set_found();

                            // Возвращаемся на хаб
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            bot.chat("/hub");
                            println!("[{}] → /hub", bot_name);
                            break;
                        } else {
                            println!(
                                "[{}] {} — {} не найден, идём дальше",
                                bot_name, anarchy_cmd, target
                            );
                        }
                    }
                });
            }
        }
        Event::Chat(chat) => {
            let msg = chat.message().to_string();
            if !msg.is_empty() {
                println!("[{}] Chat: {}", state.bot_name, msg);
            }
        }
        Event::Login => {
            println!("[{}] Login event", state.bot_name);
        }
        Event::Death(_) => {
            println!("[{}] Death event", state.bot_name);
        }
        _ => {}
    }

    Ok(())
}

/// Handler для swarm-level событий
async fn swarm_handle(
    _swarm: Swarm,
    event: SwarmEvent,
    _state: SwarmState,
) -> eyre::Result<()> {
    match &event {
        SwarmEvent::Disconnect(account, _join_opts) => {
            eprintln!("⚠️ Бот {} отключился!", account.username());
        }
        SwarmEvent::Chat(chat) => {
            let msg = chat.message().to_string();
            if !msg.is_empty() && !msg.contains("particle") {
                println!("[Swarm] Chat: {}", msg);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Проверяет таблист на наличие игрока с указанным ником
fn check_tablist_for_player(
    ecs: &azalea::ecs::world::World,
    entity: Entity,
    target_nick: &str,
) -> bool {
    // TabList — компонент HashMap<Uuid, PlayerInfo>
    let Some(tab_list) = ecs.get::<azalea::local_player::TabList>(entity) else {
        println!("  ⚠️ TabList недоступен для entity {:?}", entity);
        return false;
    };

    let target_lower = target_nick.to_lowercase();
    let mut player_count = 0;

    for (_uuid, player_info) in tab_list.iter() {
        player_count += 1;
        let name = &player_info.profile.name;
        if name.to_lowercase() == target_lower {
            println!(
                "  ✅ Найден {} (всего игроков: {})",
                name, player_count
            );
            return true;
        }
    }

    println!("  ❌ Не найден (всего игроков: {})", player_count);
    false
}

// =====================================================================
// Плагин для отклонения ресурс-паков
// =====================================================================

/// Плагин, который автоматически отклоняет все ресурс-паки от сервера.
///
/// Основан на `azalea::accept_resource_packs::AcceptResourcePacksPlugin`,
/// но отправляет `Action::Declined` вместо `Action::Accepted`.
#[derive(Clone, Default)]
pub struct DeclineResourcePacksPlugin;

impl azalea::app::Plugin for DeclineResourcePacksPlugin {
    fn build(&self, app: &mut azalea::app::App) {
        app.add_systems(azalea::app::Update, decline_resource_pack);
    }
}

fn decline_resource_pack(
    mut events: MessageReader<azalea::packet::game::ResourcePackEvent>,
    mut commands: Commands,
    query_in_config_state: Query<Option<&InConfigState>>,
) {
    use azalea::protocol::packets::{
        config,
        game::s_resource_pack::{self, ServerboundResourcePack},
    };

    for event in events.read() {
        println!(
            "📦 Resource pack (url: {}, required: {}) — ОТКЛОНЯЕМ",
            event.url, event.required
        );

        let Ok(in_config_state_option) = query_in_config_state.get(event.entity) else {
            continue;
        };

        if in_config_state_option.is_some() {
            // Config state — отправляем decline через config packet
            commands.trigger(azalea::packet::config::SendConfigPacketEvent::new(
                event.entity,
                config::ServerboundResourcePack {
                    id: event.id,
                    action: config::s_resource_pack::Action::Declined,
                },
            ));
        } else {
            // Game state — отправляем decline через game packet
            commands.trigger(azalea::packet::game::SendGamePacketEvent::new(
                event.entity,
                ServerboundResourcePack {
                    id: event.id,
                    action: s_resource_pack::Action::Declined,
                },
            ));
        }
    }
}
