use std::sync::{Arc, RwLock};
use bytes::Bytes;
use radix_trie::Trie;
use tokio::sync::broadcast::Sender;

pub type SharedState = Arc<RwLock<AppState>>;

// Global app state for axum
#[derive(Default)]
pub struct AppState {
    pub file_system: Trie<String, Arc<RwLock<IndividualVideo>>>
}

// Individual video
pub struct IndividualVideo {
    pub video_file: Vec<Bytes>,
    pub tx: Option<Sender<Bytes>>, // Will be subscribed for a reader.
    pub content_type: String, // Probably not needed, since video file server
    pub streaming: bool,
}


// Can create functions for structs here
