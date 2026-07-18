//! Generic sync TTL cache trait and in-memory implementation.
//!
//! **Design principles:**
//! - Sync (non-async) trait: appropriate for in-memory operations.
//! - Per-entry TTL with default 60s, configurable per cache instance.
//! - Max entries cap (default 10k) with LRU eviction to bound memory.
//! - Lazy expiration on `get`, no background sweeper threads.
//! - Thread-safe via a mutex-protected `HashMap`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A generic synchronous cache trait.
///
/// Provides basic key-value storage with TTL support and explicit invalidation.
/// Implementations should ensure thread safety and bounded memory usage.
pub trait Cache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Retrieve a value by key.
    ///
    /// Returns `Some(v)` if the key exists and has not expired; `None` otherwise.
    fn get(&self, key: &K) -> Option<V>;

    /// Store or update a value by key.
    fn set(&self, key: K, value: V);

    /// Invalidate a single key.
    fn invalidate(&self, key: &K);

    /// Invalidate all entries.
    fn invalidate_all(&self);
}

/// Entry metadata: value + expiration time.
#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Duration,
    last_access: u64,
}

trait CacheClock: Send + Sync {
    fn now(&self) -> Duration;
}

struct SystemClock(Instant);

impl CacheClock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}

struct CacheState<K, V> {
    store: HashMap<K, CacheEntry<V>>,
    access_counter: u64,
}

/// In-memory TTL cache with LRU eviction.
///
/// - **TTL:** Per-entry, configurable per cache instance (default 60s).
/// - **Max entries:** Capped (default 10k); when exceeded, least-recently-used entries are evicted.
/// - **Thread safety:** Serialized through a mutex-protected `HashMap`.
/// - **Expiration:** Lazily checked on `get`; expired entries are removed.
pub struct MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    state: Mutex<CacheState<K, V>>,
    ttl: Duration,
    max_entries: usize,
    clock: Arc<dyn CacheClock>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new cache with default TTL (60s) and max entries (10k).
    pub fn new() -> Self {
        Self {
            ..Self::with_ttl_and_max(Duration::from_secs(60), 10_000)
        }
    }

    /// Create a new cache with custom TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ..Self::with_ttl_and_max(ttl, 10_000)
        }
    }

    /// Create a new cache with custom TTL and max entries.
    pub fn with_ttl_and_max(ttl: Duration, max_entries: usize) -> Self {
        Self::with_clock(ttl, max_entries, Arc::new(SystemClock(Instant::now())))
    }

    fn with_clock(ttl: Duration, max_entries: usize, clock: Arc<dyn CacheClock>) -> Self {
        Self {
            state: Mutex::new(CacheState {
                store: HashMap::new(),
                access_counter: 0,
            }),
            ttl,
            max_entries,
            clock,
        }
    }

    /// Check if an entry has expired.
    fn is_expired_at(&self, entry: &CacheEntry<V>, now: Duration) -> bool {
        now.saturating_sub(entry.inserted_at) > self.ttl
    }

    /// Evict least-recently-used entry when capacity is reached.
    ///
    /// Remove the entry with the oldest access sequence.
    fn evict_lru(state: &mut CacheState<K, V>) {
        if let Some(key) = state
            .store
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| key.clone())
        {
            state.store.remove(&key);
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.state.lock().expect("cache mutex poisoned").store.len()
    }
}

impl<K, V> Default for MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Cache<K, V> for MemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn get(&self, key: &K) -> Option<V> {
        let now = self.clock.now();
        let mut state = self.state.lock().expect("cache mutex poisoned");
        let expired = state
            .store
            .get(key)
            .map(|entry| self.is_expired_at(entry, now))?;
        if expired {
            state.store.remove(key);
            return None;
        }
        state.access_counter = state.access_counter.wrapping_add(1);
        let access = state.access_counter;
        let entry = state.store.get_mut(key)?;
        entry.last_access = access;
        Some(entry.value.clone())
    }

    fn set(&self, key: K, value: V) {
        if self.max_entries == 0 {
            return;
        }
        let mut state = self.state.lock().expect("cache mutex poisoned");
        if state.store.len() >= self.max_entries && !state.store.contains_key(&key) {
            Self::evict_lru(&mut state);
        }
        state.access_counter = state.access_counter.wrapping_add(1);
        let access = state.access_counter;
        let entry = CacheEntry {
            value,
            inserted_at: self.clock.now(),
            last_access: access,
        };
        state.store.insert(key, entry);
    }

    fn invalidate(&self, key: &K) {
        self.state
            .lock()
            .expect("cache mutex poisoned")
            .store
            .remove(key);
    }

    fn invalidate_all(&self) {
        self.state
            .lock()
            .expect("cache mutex poisoned")
            .store
            .clear();
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
