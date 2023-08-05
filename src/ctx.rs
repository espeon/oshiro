use std::sync::Arc;

use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::Latency;
use twilight_http::{Client, Response};
use twilight_model::{
    application::interaction::Interaction,
    channel::Message,
    id::{marker::ApplicationMarker, Id}, http::interaction::InteractionResponse,
};

use crate::{cmd::{CommandFramework, OshiroResult}, slash::message};

#[derive(Clone)]
pub struct OshiroContext {
    pub framework: Arc<CommandFramework>,
    pub http: Arc<Client>,
    pub cache: Arc<InMemoryCache>,
    pub shard_latency: Vec<Latency>,
    pub app_id: Id<ApplicationMarker>,
}

impl OshiroContext {
    /// Shortcut function for accessing interactions
    pub fn interaction(&self) -> twilight_http::client::InteractionClient<'_> {
        self.http.interaction(self.app_id)
    }
    /// Shortcut function for sending messages with both text and slash commands
    pub async fn send_msg(
        &self,
        content: String,
        interaction_response: Option<InteractionResponse>,
        msg: Option<Box<Message>>,
        slash: Option<&Interaction>,
    ) -> OshiroResult<Option<Response<Message>>> {
        if let Some(msg) = msg {
            Ok(Some(self.http
                .create_message(msg.channel_id)
                .content(&content)?
                .await?))
        } else {
            if let Some(slash) = &slash {
                let resp = if let Some(r) = interaction_response {
                    r
                } else {
                    message(&content)
                };
                self.interaction().create_response(
                    slash.id,
                    &slash.token,
                    &resp,
                ).await?;
            }
            Ok(None)
        }
    }
}
