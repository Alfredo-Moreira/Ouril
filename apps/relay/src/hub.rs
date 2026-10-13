//! Channels in memory: one broadcast sender per channel with subscribers. Nothing is kept for
//! a channel nobody is listening to, so events for absent players are simply dropped (they
//! catch up from the server when they come back).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::broadcast;

/// Events a slow subscriber may fall behind by before missing some (they then refetch).
const CHANNEL_CAPACITY: usize = 32;

#[derive(Default)]
pub struct Hub {
    channels: Mutex<HashMap<String, broadcast::Sender<Arc<str>>>>,
}

impl Hub {
    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<Arc<str>> {
        let mut channels = self.channels.lock().expect("hub lock");
        channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .subscribe()
    }

    /// Send `event` to `channel`'s subscribers; returns how many got it.
    pub fn publish(&self, channel: &str, event: Arc<str>) -> usize {
        let mut channels = self.channels.lock().expect("hub lock");
        let Some(tx) = channels.get(channel) else {
            return 0;
        };
        match tx.send(event) {
            Ok(n) => n,
            Err(_) => {
                // Everyone left: forget the channel.
                channels.remove(channel);
                0
            }
        }
    }

    /// Drop channels whose subscribers have all gone.
    pub fn prune(&self) {
        self.channels
            .lock()
            .expect("hub lock")
            .retain(|_, tx| tx.receiver_count() > 0);
    }

    pub fn channel_count(&self) -> usize {
        self.channels.lock().expect("hub lock").len()
    }
}
