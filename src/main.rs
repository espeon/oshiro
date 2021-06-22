use futures::StreamExt;
use std::{env, error::Error, sync::Arc};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Cluster, Event, EventTypeFlags, cluster::ShardScheme};
use twilight_http::Client;

use crate::{cmd::CommandFramework, ctx::OshiroContext};

pub mod cmd;
pub mod ctx;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // .env file to the environment
    dotenv::dotenv().ok();
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a prefix in the environment.");

    let http = Client::new(&token);

    let me = http.current_user().await?;
    println!("Logged in as: {}#{}", me.name, me.discriminator);

    use twilight_model::gateway::Intents;
    let wanted_intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES;
    // wanted types of flags
    let wanted_types = EventTypeFlags::MESSAGE_CREATE | EventTypeFlags::READY;

    let (cluster, mut events) = Cluster::builder(&token, wanted_intents)
        .event_types(wanted_types)
        .shard_scheme(ShardScheme::Auto)
        .http_client(http.clone())
        .build()
        .await?;

    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    let cluster_spawn = cluster.clone();

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    let f = CommandFramework::create().await;

    let oshiro_ctx = Arc::new(
        OshiroContext{
            framework: f,
            http:http,
        }
    );

    while let Some((shard_id, event)) = events.next().await {
        // println!("New Event: {:?} | ({:?})", event.kind(), event);
        cache.update(&event);
        tokio::spawn(handle_event(shard_id, event, Arc::clone(&oshiro_ctx)));
    }

    Ok(())
}

async fn handle_event(
    shard_id: u64,
    event: Event,
    ctx: Arc<OshiroContext>
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) if msg.content.starts_with("%u") => {
            let uwued = msg
                .content
                .strip_prefix("%u")
                .unwrap_or(&msg.content)
                .to_lowercase()
                .replace("l", "w")
                .replace("r", "w")
                .replace("na", "nya");
            ctx.http.create_message(msg.channel_id).content(uwued)?.await?;
        }
        Event::MessageCreate(msg) if msg.content.starts_with("%") => {
            ctx.framework.parse_command(&"%", msg, Arc::clone(&ctx)).await?;
        }
        Event::Ready(_) => {
            println!("Shard {} is ready", shard_id)
        }
        Event::ShardConnected(_) => {
            println!("Connected on shard {}", shard_id);
        }
        _ => {}
    }

    Ok(())
}