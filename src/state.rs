use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;

/// Общий стейт между MC ботами и Telegram ботом
#[derive(Clone)]
pub struct SharedState {
    inner: Arc<SharedStateInner>,
}

struct SharedStateInner {
    /// Текущий искомый ник
    pub target_nick: Mutex<Option<String>>,
    /// Активен ли поиск
    pub search_active: AtomicBool,
    /// Найден ли игрок
    pub found: AtomicBool,
    /// Очередь анархий для проверки
    pub anarchy_queue: Mutex<VecDeque<String>>,
    /// Канал для отправки сообщений в Telegram
    pub tg_sender: Mutex<Option<mpsc::UnboundedSender<String>>>,
    /// Сколько ботов готовы (подключились и подождали)
    pub bots_ready: AtomicUsize,
    /// Общее количество ботов
    pub total_bots: usize,
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new(0)
    }
}

impl SharedState {
    pub fn new(total_bots: usize) -> Self {
        Self {
            inner: Arc::new(SharedStateInner {
                target_nick: Mutex::new(None),
                search_active: AtomicBool::new(false),
                found: AtomicBool::new(false),
                anarchy_queue: Mutex::new(VecDeque::new()),
                tg_sender: Mutex::new(None),
                bots_ready: AtomicUsize::new(0),
                total_bots,
            }),
        }
    }

    pub fn set_tg_sender(&self, sender: mpsc::UnboundedSender<String>) {
        *self.inner.tg_sender.lock() = Some(sender);
    }

    pub fn send_to_tg(&self, message: String) {
        if let Some(sender) = self.inner.tg_sender.lock().as_ref() {
            let _ = sender.send(message);
        }
    }

    pub fn start_search(&self, nick: String) {
        // Заполняем очередь анархий
        let anarchy_commands = crate::config::generate_anarchy_commands();
        let mut queue = self.inner.anarchy_queue.lock();
        queue.clear();
        for cmd in anarchy_commands {
            queue.push_back(cmd);
        }

        // Устанавливаем целевой ник
        *self.inner.target_nick.lock() = Some(nick);
        self.inner
            .found
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner
            .search_active
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn stop_search(&self) {
        self.inner
            .search_active
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.inner.anarchy_queue.lock().clear();
        *self.inner.target_nick.lock() = None;
    }

    pub fn is_search_active(&self) -> bool {
        self.inner
            .search_active
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_found(&self) -> bool {
        self.inner
            .found
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn set_found(&self) {
        self.inner
            .found
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.inner
            .search_active
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_target_nick(&self) -> Option<String> {
        self.inner.target_nick.lock().clone()
    }

    /// Берёт следующую анархию из очереди (если есть и поиск не завершён)
    pub fn take_next_anarchy(&self) -> Option<String> {
        if !self.is_search_active() {
            return None;
        }
        self.inner.anarchy_queue.lock().pop_front()
    }

    pub fn increment_ready(&self) -> usize {
        let prev = self
            .inner
            .bots_ready
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        prev + 1
    }

    pub fn all_bots_ready(&self) -> bool {
        self.inner
            .bots_ready
            .load(std::sync::atomic::Ordering::SeqCst)
            >= self.inner.total_bots
    }
}
