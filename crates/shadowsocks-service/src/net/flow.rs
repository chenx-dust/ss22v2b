//! Server flow statistic

use std::{collections::HashMap, mem, sync::{RwLock, atomic::Ordering}};

use bytes::Bytes;
use shadowsocks::config::ServerUser;

#[cfg(target_has_atomic = "64")]
type FlowCounter = std::sync::atomic::AtomicU64;
#[cfg(not(target_has_atomic = "64"))]
type FlowCounter = std::sync::atomic::AtomicU32;

/// Connection flow statistic
pub struct SingleFlowStat {
    tx: FlowCounter,
    rx: FlowCounter,
}

impl Default for SingleFlowStat {
    fn default() -> Self {
        Self {
            tx: FlowCounter::new(0),
            rx: FlowCounter::new(0),
        }
    }
}

impl SingleFlowStat {
    /// Create an empty flow statistic
    pub fn new() -> Self {
        Self::default()
    }

    /// Transmitted bytes count
    pub fn tx(&self) -> u64 {
        self.tx.swap(0, Ordering::Relaxed) as _
    }

    /// Increase transmitted bytes
    pub fn incr_tx(&self, n: u64) {
        self.tx.fetch_add(n as _, Ordering::AcqRel);
    }

    /// Received bytes count
    pub fn rx(&self) -> u64 {
        self.rx.swap(0, Ordering::Relaxed) as _
    }

    /// Increase received bytes
    pub fn incr_rx(&self, n: u64) {
        self.rx.fetch_add(n as _, Ordering::AcqRel);
    }
}

pub struct FlowStat {
    single: SingleFlowStat,
    multiple: RwLock<HashMap<Bytes, SingleFlowStat>>,
}

impl Default for FlowStat {
    fn default() -> Self {
        Self {
            single: SingleFlowStat::new(),
            multiple: RwLock::new(HashMap::new()),
        }
    }
}

impl FlowStat {
    /// Create an empty flow statistic
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase transmitted bytes
    pub fn incr_tx(&self, n: u64, user: Option<&ServerUser>) {
        self.single.tx.fetch_add(n as _, Ordering::AcqRel);
        if let Some(user) = user {
            let key = user.identity_hash();
            if let Some(stat) = self.multiple.read().expect("multiple flow stat poisoned").get(key) {
                stat.tx.fetch_add(n as _, Ordering::AcqRel);
            } else {
                self.multiple.write().expect("multiple flow stat poisoned")
                    .entry(key.to_owned().into()).or_default()
                    .tx.fetch_add(n as _, Ordering::AcqRel);
            }
        }
    }

    /// Increase received bytes
    pub fn incr_rx(&self, n: u64, user: Option<&ServerUser>) {
        self.single.rx.fetch_add(n as _, Ordering::AcqRel);
        if let Some(user) = user {
            let key = user.identity_hash();
            if let Some(stat) = self.multiple.read().expect("multiple flow stat poisoned").get(key) {
                stat.rx.fetch_add(n as _, Ordering::AcqRel);
            } else {
                self.multiple.write().expect("multiple flow stat poisoned")
                    .entry(key.to_owned().into()).or_default()
                    .rx.fetch_add(n as _, Ordering::AcqRel);
            }
        }
    }

    pub fn get_single(&self) -> &SingleFlowStat {
        &self.single
    }

    pub fn get_multiple(&self) -> HashMap<Bytes, SingleFlowStat> {
        // Drain the collected per-user stats without moving the lock itself
        let mut guard = self.multiple.write().expect("multiple flow stat poisoned");
        mem::take(&mut *guard)
    }
}
