use std::sync::Arc;

use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::Latency;
use twilight_http::Client;

use crate::cmd::CommandFramework;

#[derive(Clone)]
pub struct OshiroContext {
    pub framework: Arc<CommandFramework>,
    pub http: Arc<Client>,
    pub cache: Arc<InMemoryCache>,
    pub shard_latency: Vec<Latency>
}