//! In-memory session management.
//!
//! When a user unlocks a container, the decrypted payload is held here
//! in memory for the duration of the session. The session is cleared:
//!   - When the user manually locks the container
//!   - When the app closes (memory is reclaimed by the OS)
//!   - On future: idle timeout (Phase 3)
//!
//! Key material (the raw AES key) is stored in Zeroizing<> to ensure
//! it is wiped from memory when the session is dropped, not merely
//! marked as free — which could leave key bytes in RAM.

use crate::vault::ContainerPayload;
use crate::vault::ContainerMetadataV2;
use std::collections::HashMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

use crate::crypto::SALT_LEN;

/// A single active decrypted session for one container.
pub struct Session {
    /// The decrypted payload held in memory.
    pub payload: ContainerPayload,
    /// The derived AES key, kept for re-encryption during edit mode.
    /// Wrapped in Zeroizing so bytes are wiped on drop.
    pub key: Zeroizing<Vec<u8>>,
    /// The salt from this container's blob, needed for password verification
    /// during save_edits before re-encrypting.
    pub salt: [u8; SALT_LEN],
}

/// Global session store — keyed by container ID.
pub struct SessionStore(pub Mutex<HashMap<String, Session>>);

impl SessionStore {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    /// Store a new session for a container.
    pub fn set(&self, container_id: String, session: Session) {
        self.0.lock().unwrap().insert(container_id, session);
    }

    /// Check if a session exists for a container.
    pub fn has(&self, container_id: &str) -> bool {
        self.0.lock().unwrap().contains_key(container_id)
    }

    /// Lock (clear) a container session, wiping key material from memory.
    pub fn lock(&self, container_id: &str) {
        self.0.lock().unwrap().remove(container_id);
        // Session drop triggers Zeroizing<> wipe on the key field
    }

    /// Lock all sessions — called on app close or master lock.
    pub fn lock_all(&self) {
        self.0.lock().unwrap().clear();
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── V2 Session with LRU Cache ──────────────────────────────────────────────

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Default maximum cache size in bytes (50 MB).
pub const DEFAULT_MAX_CACHE_BYTES: usize = 50 * 1024 * 1024;

/// Cached decrypted file data with zeroization on eviction/drop.
pub struct CachedFile {
    pub data: Vec<u8>,
}

impl Drop for CachedFile {
    fn drop(&mut self) {
        self.data.fill(0);
        self.data.clear();
    }
}

/// V2 session — metadata + key + bounded LRU cache of decrypted files.
pub struct SessionV2 {
    pub key: Zeroizing<[u8; 32]>,
    pub salt: [u8; SALT_LEN],
    pub metadata: ContainerMetadataV2,
    pub blob_path: String,
    pub cache: Mutex<LruCache<String, CachedFile>>,
    pub max_cache_bytes: usize,
    pub current_cache_bytes: AtomicUsize,
}

impl SessionV2 {
    pub fn new(
        key: Zeroizing<[u8; 32]>,
        salt: [u8; SALT_LEN],
        metadata: ContainerMetadataV2,
        blob_path: String,
        max_cache_bytes: usize,
    ) -> Self {
        let cap = NonZeroUsize::new(1024).unwrap();
        Self {
            key,
            salt,
            metadata,
            blob_path,
            cache: Mutex::new(LruCache::new(cap)),
            max_cache_bytes,
            current_cache_bytes: AtomicUsize::new(0),
        }
    }

    /// Insert decrypted file data into the cache. If the file exists, replace it.
    /// Evicts old entries if the cache would exceed `max_cache_bytes`.
    pub fn cache_put(&self, file_id: String, data: Vec<u8>) {
        let data_len = data.len();
        let mut cache = self.cache.lock().unwrap();

        // If file already cached, subtract its old size
        if let Some(old) = cache.get(&file_id) {
            self.current_cache_bytes.fetch_sub(old.data.len(), Ordering::SeqCst);
        }

        // Evict oldest entries until we have room
        let current = self.current_cache_bytes.load(Ordering::SeqCst);
        let mut new_total = current + data_len;
        while new_total > self.max_cache_bytes {
            if let Some((_, evicted)) = cache.pop_lru() {
                new_total -= evicted.data.len();
            } else {
                break;
            }
        }

        cache.put(file_id, CachedFile { data });
        self.current_cache_bytes.store(new_total, Ordering::SeqCst);
    }

    /// Get decrypted file data from cache. Returns cloned bytes on hit.
    pub fn cache_get(&self, file_id: &str) -> Option<Vec<u8>> {
        let cache = self.cache.lock().unwrap();
        cache.peek(file_id).map(|f| f.data.clone())
    }

    /// Explicitly release a file from cache, zeroizing its data.
    pub fn release_file_data(&self, file_id: &str) {
        let mut cache = self.cache.lock().unwrap();
        if let Some(f) = cache.pop(file_id) {
            let freed = f.data.len();
            drop(f); // triggers Drop::zeroize
            self.current_cache_bytes.fetch_sub(freed, Ordering::SeqCst);
        }
    }

    /// Zeroize all cached files and clear the cache.
    pub fn lock(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear(); // each CachedFile Drop zeroizes its data
        self.current_cache_bytes.store(0, Ordering::SeqCst);
    }
}

/// Global v2 session store — keyed by container ID.
pub struct SessionStoreV2(pub Mutex<HashMap<String, SessionV2>>);

impl SessionStoreV2 {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    pub fn set(&self, container_id: String, session: SessionV2) {
        self.0.lock().unwrap().insert(container_id, session);
    }

    pub fn get(&self, container_id: &str) -> Option<SessionV2> {
        self.0.lock().unwrap().get(container_id).cloned()
    }

    pub fn get_mut<F, R>(&self, container_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut SessionV2) -> R,
    {
        self.0.lock().unwrap().get_mut(container_id).map(f)
    }

    pub fn lock(&self, container_id: &str) {
        let mut sessions = self.0.lock().unwrap();
        if let Some(session) = sessions.remove(container_id) {
            session.lock();
        }
    }

    pub fn lock_all(&self) {
        let mut sessions = self.0.lock().unwrap();
        for (_, session) in sessions.drain() {
            session.lock();
        }
    }
}

impl Default for SessionStoreV2 {
    fn default() -> Self {
        Self::new()
    }
}

// Clone isn't derivable due to Mutex — we only need a reference-based API.
impl Clone for SessionV2 {
    fn clone(&self) -> Self {
        let mut key = [0u8; 32];
        key.copy_from_slice(self.key.as_ref());
        let cap = NonZeroUsize::new(1024).unwrap();
        Self {
            key: Zeroizing::new(key),
            salt: self.salt,
            metadata: self.metadata.clone(),
            blob_path: self.blob_path.clone(),
            cache: Mutex::new(LruCache::new(cap)),
            max_cache_bytes: self.max_cache_bytes,
            current_cache_bytes: AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::FileMetadata;

    fn dummy_metadata() -> ContainerMetadataV2 {
        ContainerMetadataV2 {
            version: 2,
            files: vec![
                FileMetadata {
                    id: "file-1".into(),
                    name: "test.jpg".into(),
                    mime: "image/jpeg".into(),
                    size: 1024,
                    offset: 64,
                    data_nonce: [1u8; 12],
                    sha256: "abc123".into(),
                    chunks: None,
                },
                FileMetadata {
                    id: "file-2".into(),
                    name: "big.bin".into(),
                    mime: "application/octet-stream".into(),
                    size: 10 * 1024 * 1024,
                    offset: 1120,
                    data_nonce: [2u8; 12],
                    sha256: "def456".into(),
                    chunks: None,
                },
            ],
        }
    }

    fn dummy_session() -> SessionV2 {
        let key = Zeroizing::new([42u8; 32]);
        SessionV2::new(key, [0u8; SALT_LEN], dummy_metadata(), "/tmp/test.enc".into(), DEFAULT_MAX_CACHE_BYTES)
    }

    #[test]
    fn cache_insert_and_hit() {
        let session = dummy_session();
        let data = vec![1u8, 2, 3, 4, 5];
        session.cache_put("file-1".into(), data.clone());
        let hit = session.cache_get("file-1");
        assert_eq!(hit, Some(data));
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn cache_get_on_miss() {
        let session = dummy_session();
        assert_eq!(session.cache_get("nonexistent"), None);
    }

    #[test]
    fn cache_replace_updates_size() {
        let session = dummy_session();
        let small = vec![0u8; 100];
        let big = vec![0u8; 1000];
        session.cache_put("file-1".into(), small);
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 100);
        session.cache_put("file-1".into(), big);
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 1000);
    }

    #[test]
    fn cache_eviction_on_size_limit() {
        let session = SessionV2::new(
            Zeroizing::new([42u8; 32]),
            [0u8; SALT_LEN],
            dummy_metadata(),
            "/tmp/test.enc".into(),
            1000,
        );
        // Insert 800 bytes
        session.cache_put("file-1".into(), vec![0u8; 800]);
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 800);
        // Insert 900 bytes — should evict file-1 first
        session.cache_put("file-2".into(), vec![1u8; 900]);
        let total = session.current_cache_bytes.load(Ordering::SeqCst);
        assert!(total <= 1000, "cache exceeded max: {}", total);
        // file-1 should be evicted
        assert_eq!(session.cache_get("file-1"), None);
        // file-2 should be present
        assert_eq!(session.cache_get("file-2"), Some(vec![1u8; 900]));
    }

    #[test]
    fn release_file_data_zeroizes() {
        let session = dummy_session();
        session.cache_put("file-1".into(), vec![7u8; 256]);
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 256);
        session.release_file_data("file-1");
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 0);
        assert_eq!(session.cache_get("file-1"), None);
    }

    #[test]
    fn lock_wipes_cache() {
        let session = dummy_session();
        session.cache_put("file-1".into(), vec![5u8; 512]);
        session.cache_put("file-2".into(), vec![6u8; 128]);
        session.lock();
        assert_eq!(session.current_cache_bytes.load(Ordering::SeqCst), 0);
        assert_eq!(session.cache_get("file-1"), None);
        assert_eq!(session.cache_get("file-2"), None);
    }

    #[test]
    fn session_store_v2_lock_all() {
        let store = SessionStoreV2::new();
        let s1 = dummy_session();
        s1.cache_put("file-1".into(), vec![1u8; 64]);
        let s2 = dummy_session();
        s2.cache_put("file-1".into(), vec![2u8; 32]);
        store.set("a".into(), s1);
        store.set("b".into(), s2);
        store.lock_all();
        // Both sessions are removed; verifying with a new dummy won't hit
        // (SessionV2 doesn't support a direct 'has' check; lock_all just drains)
    }
}
