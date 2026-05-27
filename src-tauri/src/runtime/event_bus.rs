use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ActiveAppChanged,
    BrowserUrlChanged,
    SlackContextChanged,
    UserIdle,
    UserTyping,
    PetStatusChanged,
    ModuleLoaded,
    ModuleReaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeowEvent {
    pub event_type: EventType,
    pub source: String,
    pub payload: HashMap<String, String>,
    pub timestamp: u64,
}

impl MeowEvent {
    pub fn new(event_type: EventType, source: &str) -> Self {
        Self {
            event_type,
            source: source.to_string(),
            payload: HashMap::new(),
            timestamp: now_ms(),
        }
    }

    pub fn with_payload(mut self, key: &str, value: &str) -> Self {
        self.payload.insert(key.to_string(), value.to_string());
        self
    }
}

use crate::util::now_ms;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<MeowEvent>,
    history: Arc<RwLock<Vec<MeowEvent>>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn publish(&self, event: MeowEvent) {
        {
            let mut history = self.history.write().await;
            history.push(event.clone());
            if history.len() > 1000 {
                history.drain(0..500);
            }
        }
        let _ = self.sender.send(event);
    }

    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<MeowEvent> {
        self.sender.subscribe()
    }

    #[allow(dead_code)]
    pub fn subscribe_filtered(&self, event_types: Vec<EventType>) -> FilteredReceiver {
        FilteredReceiver {
            inner: self.sender.subscribe(),
            filter: event_types,
        }
    }

    #[allow(dead_code)]
    pub async fn recent_events(&self, limit: usize) -> Vec<MeowEvent> {
        let history = self.history.read().await;
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }
}

#[allow(dead_code)]
pub struct FilteredReceiver {
    inner: broadcast::Receiver<MeowEvent>,
    filter: Vec<EventType>,
}

#[allow(dead_code)]
impl FilteredReceiver {
    pub async fn recv(&mut self) -> Result<MeowEvent, broadcast::error::RecvError> {
        loop {
            let event = self.inner.recv().await?;
            if self.filter.contains(&event.event_type) {
                return Ok(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_and_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        let event = MeowEvent::new(EventType::UserTyping, "system")
            .with_payload("app", "VSCode");

        bus.publish(event.clone()).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type, EventType::UserTyping);
        assert_eq!(received.source, "system");
        assert_eq!(received.payload.get("app").unwrap(), "VSCode");
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = MeowEvent::new(EventType::ActiveAppChanged, "system");
        bus.publish(event).await;

        assert_eq!(rx1.recv().await.unwrap().event_type, EventType::ActiveAppChanged);
        assert_eq!(rx2.recv().await.unwrap().event_type, EventType::ActiveAppChanged);
    }

    #[tokio::test]
    async fn test_event_history() {
        let bus = EventBus::new(16);

        for i in 0..5 {
            let event = MeowEvent::new(EventType::UserTyping, "system")
                .with_payload("count", &i.to_string());
            bus.publish(event).await;
        }

        let recent = bus.recent_events(3).await;
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].payload.get("count").unwrap(), "2");
        assert_eq!(recent[2].payload.get("count").unwrap(), "4");
    }

    #[tokio::test]
    async fn test_history_cap() {
        let bus = EventBus::new(2048);

        for i in 0..1200 {
            let event = MeowEvent::new(EventType::UserTyping, "test")
                .with_payload("i", &i.to_string());
            bus.publish(event).await;
        }

        let history = bus.history.read().await;
        assert!(history.len() <= 1000);
    }

    #[tokio::test]
    async fn test_payload_builder() {
        let event = MeowEvent::new(EventType::BrowserUrlChanged, "browser")
            .with_payload("site", "github.com")
            .with_payload("title", "Pull Request #42");

        assert_eq!(event.payload.len(), 2);
        assert_eq!(event.payload.get("site").unwrap(), "github.com");
        assert_eq!(event.payload.get("title").unwrap(), "Pull Request #42");
    }
}
