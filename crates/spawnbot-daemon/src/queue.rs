use spawnbot_common::types::Priority;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use tokio::sync::{Mutex, Notify};

#[derive(Debug, Clone)]
pub enum EventSource {
    User { reply_to: ReplyTarget },
    Cron(String),
    Idle,
    Poller(String),
    SessionRotation,
}

impl std::fmt::Display for EventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User { .. } => write!(f, "user"),
            Self::Cron(name) => write!(f, "cron:{}", name),
            Self::Idle => write!(f, "idle"),
            Self::Poller(name) => write!(f, "poller:{}", name),
            Self::SessionRotation => write!(f, "session_rotation"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    System,
}

#[derive(Debug, Clone)]
pub enum ReplyTarget {
    Telegram(i64),
    Tui,
    Desktop,
}

pub struct QueueEvent {
    pub priority: Priority,
    pub content: String,
    pub source: EventSource,
    pub role: MessageRole,
}

impl QueueEvent {
    pub fn new(priority: Priority, content: String) -> Self {
        Self {
            priority,
            content,
            source: EventSource::Idle,
            role: MessageRole::System,
        }
    }

    pub fn user(content: String, reply_to: ReplyTarget) -> Self {
        Self {
            priority: Priority::High,
            content,
            source: EventSource::User { reply_to },
            role: MessageRole::User,
        }
    }

    pub fn system(priority: Priority, content: String, source: EventSource) -> Self {
        Self {
            priority,
            content,
            source,
            role: MessageRole::System,
        }
    }
}

impl Eq for QueueEvent {}

impl PartialEq for QueueEvent {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl PartialOrd for QueueEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

pub struct PriorityQueue {
    heap: Mutex<BinaryHeap<QueueEvent>>,
    notify: Notify,
}

impl PriorityQueue {
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            notify: Notify::new(),
        }
    }

    pub async fn enqueue(&self, event: QueueEvent) {
        self.heap.lock().await.push(event);
        self.notify.notify_one();
    }

    pub async fn dequeue(&self) -> QueueEvent {
        loop {
            {
                // scope the lock
                let mut heap = self.heap.lock().await;
                if let Some(event) = heap.pop() {
                    return event;
                }
            }
            self.notify.notified().await;
        }
    }

    pub async fn len(&self) -> usize {
        self.heap.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = PriorityQueue::new();
        queue
            .enqueue(QueueEvent::new(Priority::Low, "low".into()))
            .await;
        queue
            .enqueue(QueueEvent::new(Priority::High, "high".into()))
            .await;
        queue
            .enqueue(QueueEvent::new(Priority::Normal, "normal".into()))
            .await;

        let first = queue.dequeue().await;
        assert_eq!(first.priority, Priority::High);
        assert_eq!(first.content, "high");

        let second = queue.dequeue().await;
        assert_eq!(second.priority, Priority::Normal);
        assert_eq!(second.content, "normal");

        let third = queue.dequeue().await;
        assert_eq!(third.priority, Priority::Low);
        assert_eq!(third.content, "low");
    }

    #[tokio::test]
    async fn test_queue_len() {
        let queue = PriorityQueue::new();
        assert_eq!(queue.len().await, 0);

        queue
            .enqueue(QueueEvent::new(Priority::Normal, "a".into()))
            .await;
        queue
            .enqueue(QueueEvent::new(Priority::High, "b".into()))
            .await;
        assert_eq!(queue.len().await, 2);

        let _ = queue.dequeue().await;
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_user_event_is_high_priority() {
        let event = QueueEvent::user("hello".into(), ReplyTarget::Tui);
        assert_eq!(event.priority, Priority::High);
        assert!(matches!(event.role, MessageRole::User));
    }

    #[tokio::test]
    async fn test_system_event_custom_priority() {
        let event = QueueEvent::system(
            Priority::Critical,
            "urgent".into(),
            EventSource::Cron("test".into()),
        );
        assert_eq!(event.priority, Priority::Critical);
        assert!(matches!(event.role, MessageRole::System));
    }
}
