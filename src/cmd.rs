use std::{collections::HashMap, error::Error, pin::Pin, sync::Arc};

use twilight_http::Client;

use futures::Future;

use crate::ctx::OshiroContext;

use chrono::DateTime;
use chrono::Utc;

type OshiroResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

pub type CommandFn = Box<dyn Fn(CommandContext) -> CommandResultOuter + Send + Sync>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = OshiroResult> + Send>>;

pub struct CommandInstance {
    pub name: String,
    pub description: String,
    pub exec: CommandFn,
}

pub struct CommandContext {
    oshiro: Arc<OshiroContext>,
    msg: Box<twilight_model::gateway::payload::MessageCreate>,
}
#[derive(Default)]
pub struct CommandFramework {
    commands: HashMap<String, Arc<CommandInstance>>,
}

#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! pin_box {
        ($e: expr) => {
            Box::new(move |ctx| Box::pin($e(ctx)))
        };
    }

    #[macro_export]
    macro_rules! cmd {
        ($func: ident, $name: expr, $desc: expr) => {{
            Arc::new(CommandInstance {
                name: $name,
                description: $desc,
                exec: Box::new(move |ctx| Box::pin($func(ctx))),
            })
        }};
    }
}

impl CommandFramework {
    pub async fn create() -> OshiroResult<Self> {
        let mut f = CommandFramework::default();
        f.add_command(cmd!(
            test_commander,
            "test".to_string(),
            "test command".to_string()
        ))
        .await?;
        f.add_command(cmd!(ping, "ping".to_string(), "just 4 fun".to_string()))
            .await?;
        Ok(f)
    }

    pub async fn add_command(&mut self, cmd: Arc<CommandInstance>) -> OshiroResult<()> {
        self.commands.insert(cmd.name.clone(), cmd);
        Ok(())
    }

    pub async fn parse_command(
        &self,
        prefix: &str,
        msg: Box<twilight_model::gateway::payload::MessageCreate>,
        ctx: Arc<OshiroContext>,
    ) -> OshiroResult<()> {
        dbg!(prefix);

        let message = msg.content.strip_prefix(prefix).unwrap_or(&msg.content);
        dbg!(message);
        dbg!(self.commands.keys());
        if message.starts_with("hello") {
            test_command(msg, ctx.http.clone()).await?
        } else if self.commands.contains_key(message) {
            let v = self.commands.get(message).unwrap();
            let cctx = CommandContext {
                oshiro: Arc::clone(&ctx),
                msg,
            };
            (v.exec)(cctx).await?;
        }
        Ok(())
    }
}

async fn test_commander(ctx: CommandContext) -> OshiroResult<()> {
    let a = &ctx.msg.0.author.name;
    ctx.oshiro
        .http
        .create_message(ctx.msg.channel_id)
        .content(format!("hello {}, from Test Commander", a))?
        .await?;
    Ok(())
}
struct Timer {
    start: DateTime<Utc>,
}

impl Timer {
    pub fn new() -> Self {
        Timer { start: Utc::now() }
    }

    pub fn elapsed_ms(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.start)
            .num_milliseconds()
    }
}
async fn ping(ctx: CommandContext) -> OshiroResult<()> {
    let timer = Timer::new();
    let sent_msg = ctx
        .oshiro
        .http
        .create_message(ctx.msg.channel_id)
        .content(format!("ping!!!"))?
        .await?;
    let msg_ms = timer.elapsed_ms();

    let mut shard_latencies = vec![];

    for shard in ctx.oshiro.cluster.shards() {
        let info = shard.info()?;
        let avg = match info.latency().average() {
            Some(x) =>         format!(
                "{:.3}",
                x.as_secs() as f64 / 1000.0 + f64::from(x.subsec_nanos()) * 1e-6
            ),
            None => "N/A".to_string(),
        };
        shard_latencies.push(format!("shard id {} - ws avg: {}ms - hb: {}", info.id(), avg, info.latency().heartbeats()))
    }

    let e = ctx.oshiro
    .http
    .update_message(sent_msg.channel_id, sent_msg.id)
    .content(format!("pong!\nhttp: {}ms\n{}", msg_ms, shard_latencies.join("\n")))?;

e.await?;

    Ok(())
}

async fn test_command(
    msg: Box<twilight_model::gateway::payload::MessageCreate>,
    http: Client,
) -> OshiroResult<()> {
    let a = &msg.0.author.name;
    http.create_message(msg.channel_id)
        .content(format!("hello {}", a))?
        .await?;
    Ok(())
}
