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
