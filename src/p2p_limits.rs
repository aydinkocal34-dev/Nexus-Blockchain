use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};

/// Maximum number of simultaneously tracked peer connections.
pub const MAX_PEER_CONNECTIONS: usize = 128;
/// Maximum number of simultaneous connections from one IP address.
pub const MAX_CONNECTIONS_PER_IP: usize = 4;

#[derive(Debug, Clone)]
pub struct ConnectionManager {
    inner: Arc<Mutex<ConnectionManagerInner>>,
}

#[derive(Debug, Default)]
struct ConnectionManagerInner {
    total_connections: usize,
    ip_counts: HashMap<IpAddr, usize>,
}

/// A lease on one accepted connection. Dropping it releases both counters.
#[derive(Debug)]
pub struct ConnectionGuard {
    manager: Arc<Mutex<ConnectionManagerInner>>,
    ip: IpAddr,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ConnectionManagerInner::default())),
        }
    }

    fn lock(&self) -> MutexGuard<'_, ConnectionManagerInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Reserves a connection slot atomically, or returns None when a limit is reached.
    pub fn try_accept(&self, ip: IpAddr) -> Option<ConnectionGuard> {
        let mut inner = self.lock();

        if inner.total_connections >= MAX_PEER_CONNECTIONS {
            return None;
        }

        let ip_count = inner.ip_counts.get(&ip).copied().unwrap_or(0);
        if ip_count >= MAX_CONNECTIONS_PER_IP {
            return None;
        }

        inner.total_connections += 1;
        *inner.ip_counts.entry(ip).or_insert(0) += 1;

        Some(ConnectionGuard {
            manager: Arc::clone(&self.inner),
            ip,
        })
    }

    pub fn total_connections(&self) -> usize {
        self.lock().total_connections
    }

    pub fn ip_connection_count(&self, ip: &IpAddr) -> usize {
        self.lock().ip_counts.get(ip).copied().unwrap_or(0)
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        // Recover a poisoned mutex so dropping a guard still attempts cleanup.
        let mut inner = self
            .manager
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        inner.total_connections = inner.total_connections.saturating_sub(1);
        if let Some(count) = inner.ip_counts.get_mut(&self.ip) {
            if *count > 1 {
                *count -= 1;
            } else {
                inner.ip_counts.remove(&self.ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::thread;

    fn ip(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        Ipv4Addr::new(a, b, c, d).into()
    }

    #[test]
    fn per_ip_limit_and_raii_cleanup() {
        let manager = ConnectionManager::new();
        let address = ip(192, 168, 1, 50);
        let mut guards = Vec::new();

        for _ in 0..MAX_CONNECTIONS_PER_IP {
            guards.push(manager.try_accept(address).expect("first four accepted"));
        }
        assert!(manager.try_accept(address).is_none());
        assert_eq!(manager.ip_connection_count(&address), 4);
        assert_eq!(manager.total_connections(), 4);

        drop(guards.pop());
        assert_eq!(manager.ip_connection_count(&address), 3);
        assert!(manager.try_accept(address).is_some());
    }

    #[test]
    fn ipv6_is_supported() {
        let manager = ConnectionManager::new();
        let address: IpAddr = Ipv6Addr::LOCALHOST.into();
        let guard = manager.try_accept(address).expect("connection accepted");
        assert_eq!(manager.ip_connection_count(&address), 1);
        drop(guard);
        assert_eq!(manager.total_connections(), 0);
        assert_eq!(manager.ip_connection_count(&address), 0);
    }

    #[test]
    fn total_limit_is_enforced() {
        let manager = ConnectionManager::new();
        let mut guards = Vec::new();

        for i in 0..32u8 {
            let address = ip(172, 16, 0, i);
            for _ in 0..MAX_CONNECTIONS_PER_IP {
                guards.push(manager.try_accept(address).expect("slot available"));
            }
        }

        assert_eq!(manager.total_connections(), MAX_PEER_CONNECTIONS);
        assert!(manager.try_accept(ip(127, 0, 0, 1)).is_none());
    }

    #[test]
    fn concurrent_attempts_cannot_exceed_per_ip_limit() {
        let manager = ConnectionManager::new();
        let address = ip(10, 0, 0, 1);
        let mut threads = Vec::new();

        for _ in 0..16 {
            let manager = manager.clone();
            threads.push(thread::spawn(move || manager.try_accept(address)));
        }

        let guards: Vec<_> = threads
            .into_iter()
            .filter_map(|t| t.join().unwrap())
            .collect();
        assert_eq!(guards.len(), MAX_CONNECTIONS_PER_IP);
        assert_eq!(manager.total_connections(), MAX_CONNECTIONS_PER_IP);
    }
}
