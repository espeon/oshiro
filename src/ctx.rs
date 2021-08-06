use twilight_gateway::Cluster;
use twilight_http::Client;

use crate::cmd::CommandFramework;

pub struct OshiroContext {
    pub framework: CommandFramework,
    pub http: Client,
    pub cluster: Cluster
}