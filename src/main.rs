use cmd::OshiroResult;
use futures::StreamExt;
use std::{
    env,
    error::Error,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{
    stream::{self, ShardEventStream},
    Config, ConfigBuilder, Event, EventTypeFlags, Intents, ShardId,
};
use twilight_http::Client as HttpClient;

use crate::{cmd::CommandFramework, ctx::OshiroContext};

pub mod cmd;
pub mod commands;
pub mod ctx;
pub mod helper;
pub mod slash;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // .env file to the environment
    dotenv::dotenv().ok();
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token =
        env::var("DISCORD_TOKEN").expect("Expected a Discord bot token in the environment.");

    let http = Arc::new(HttpClient::new(token.clone()));
    let me = http.current_user().await?.model().await?;

    tracing::info!("Logged in as {}#{}", me.name, me.discriminator);

    let current_app = http.current_user_application().await?.model().await?;

    let interaction = http.interaction(current_app.id);

    interaction
        .set_global_commands(
            &slash::commands()
                .into_iter()
                .map(|c| c.1.command)
                .collect::<Vec<twilight_model::application::command::Command>>(),
        )
        .await?;

    let intents =
        Intents::MESSAGE_CONTENT | Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS;
    let flags = EventTypeFlags::MESSAGE_CREATE
        | EventTypeFlags::READY
        | EventTypeFlags::INTERACTION_CREATE
        | EventTypeFlags::INTERACTION_CREATE;

    let builder = Config::builder(token.clone(), intents)
        .event_types(flags)
        .build();

    fn builder_callback(_: ShardId, c: ConfigBuilder) -> Config {
        c.build()
    }

    let mut shards = stream::create_recommended(&http, builder, builder_callback)
        .await?
        .collect::<Vec<_>>();

    //let shard_closers: Vec<MessageSender> = shards.iter().map(Shard::sender).collect();
    //tokio::spawn(async move {
    //    tokio::signal::ctrl_c()
    //        .await
    //        .expect("Failed to listen to ctrl-c");
    //    println!("Shutting down...");
    //    for shard in shard_closers {
    //        print!(".");
    //        shard.close(CloseFrame::NORMAL).ok();
    //    }
    //});

    tracing::trace!("{} shard(s) in the event stream", shards.len());

    let cache = InMemoryCache::builder()
        .resource_types(
            ResourceType::MESSAGE
                | ResourceType::CHANNEL
                | ResourceType::MEMBER
                | ResourceType::GUILD
        )
        .build();

    let framework = Arc::new(CommandFramework::create().await?);

    let arc_cache = Arc::new(cache);
    let mut latency = Vec::new();
    let latency_last_checked = SystemTime::now();

    for s in shards.iter() {
        tracing::info!("Checking latency");
        latency.push(s.latency().clone());
    }

    let oshiro_ctx = Arc::new(Mutex::new(OshiroContext {
        framework: Arc::clone(&framework),
        http,
        cache: arc_cache,
        shard_latency: latency,
        app_id: current_app.id,
    }));

    let mut event_stream = ShardEventStream::new(shards.iter_mut());

    // Event loop -
    while let Some((shard, event_result)) = event_stream.next().await {
        if SystemTime::now() > latency_last_checked + Duration::from_secs(30) {
            tracing::info!("Checking latency");
            if let Some(l) = oshiro_ctx
                .lock()
                .await
                .shard_latency
                .get_mut(shard.id().number() as usize)
            {
                let g = shard.latency().to_owned();
                *l = g
            }
        }
        let event = match event_result {
            Ok(e) => e,
            Err(source) => {
                if source.is_fatal() {
                    tracing::error!(?source, "fatal error receiving event:");
                    break;
                } else {
                    tracing::warn!(?source, "error receiving event:");
                }

                continue;
            }
        };

        if let Err(e) = handle_event(
            event,
            Arc::clone(&oshiro_ctx),
            Arc::clone(&framework),
            me.id.to_string(),
        )
        .await
        {
            tracing::error!("Handler error: {e}");
        }
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    ctx: Arc<Mutex<OshiroContext>>,
    framework: Arc<CommandFramework>,
    me: String,
) -> OshiroResult<()> {
    // TODO: move good_bots to somewhere else
    let good_bots: Vec<String> = Vec::new();
    let prefix = env::var("PREFIX").expect("Expected a prefix in the environment.");
    match event {
        Event::MessageCreate(msg)
            if msg.author.bot && !good_bots.contains(&msg.author.id.to_string()) =>
        {
            return Ok(())
        }
        Event::MessageCreate(msg) if msg.content.starts_with(&prefix) => {
            framework
                .parse_command(&prefix, msg, Arc::clone(&ctx))
                .await?;
        }
        Event::MessageCreate(msg) => {
            if msg.content == format!("<@{}>", me) {
                let context = ctx.lock().await;
                context.http
                    .create_message(msg.channel_id)
                    .content(&format!("My prefix is {}, but you can ping me as well.", &prefix))?
                    .await?;
            }
        }
        Event::InteractionCreate(slash) => slash::handle(slash.0, Arc::clone(&ctx)).await?,
        Event::Ready(r) => {
            if let Some(s) = r.shard {
                tracing::info!("Shard {} is ready", s.number())
            }
        }
        _ => {}
    }

    Ok(())
}
