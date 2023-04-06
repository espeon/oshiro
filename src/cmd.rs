use std::{collections::HashMap, error::Error, pin::Pin, sync::Arc};

use futures::Future;
use tokio::sync::Mutex;

use crate::ctx::OshiroContext;

use chrono::DateTime;
use chrono::Utc;

type OshiroResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

pub type CommandFn = Box<dyn Fn(CommandContext) -> CommandResultOuter + Send + Sync>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = OshiroResult> + Send>>;

/// Describes a command
pub struct CommandInstance {
    pub name: String,
    pub description: String,
    pub exec: CommandFn,
}

/// A struct passed to the command when launched
pub struct CommandContext {
    oshiro: Arc<Mutex<OshiroContext>>,
    msg: Box<twilight_model::gateway::payload::incoming::MessageCreate>,
    stripped: String
}
#[derive(Default, Clone)]
pub struct CommandFramework {
    commands: HashMap<String, Arc<CommandInstance>>,
}

#[macro_use]
pub mod macros {
    #[macro_export]
    /// Easy way to make a CommandInstance to add to the CommandFramework's commands.
    ///
    /// Name and description need to be formattable, i.e. be able to be passed into a basic format!() macro
    /// ```
    /// // note: no parentheses in function call
    /// cmd!(framework, function, "name", "description");
    /// ```
    macro_rules! cmd {
        ($framework: ident, $func: ident, $name: expr, $desc: expr) => {{
            $framework.add_command(Arc::new(CommandInstance {
                name: format!("{}", $name),
                description: format!("{}", $desc),
                exec: Box::new(move |ctx| Box::pin($func(ctx))),
            }))
        }};
    }
}

impl CommandFramework {
    /// Make a CommandFramework
    pub async fn create() -> OshiroResult<Self> {
        let mut f = CommandFramework::default();
        cmd!(
            f,
            hi_echo,
            "hi".to_string(),
            "will respond with hi".to_string()
        )
        .await?;
        cmd!(f, ping, "ping", "just 4 fun").await?;
        (cmd!(f, uwu, "uwu", "uwuifies strings xD")).await?;
        Ok(f)
    }

    /// Add a command. Internally used in the "cmd" macro.
    pub async fn add_command(&mut self, cmd: Arc<CommandInstance>) -> OshiroResult<()> {
        self.commands.insert(cmd.name.clone(), cmd);
        Ok(())
    }

    /// Will parse a command, and launch a command with a CommandContext and a MessageCreate object.
    pub async fn parse_command(
        &self,
        prefix: &str,
        msg: Box<twilight_model::gateway::payload::incoming::MessageCreate>,
        ctx: Arc<Mutex<OshiroContext>>,
    ) -> OshiroResult<()> {
        tracing::trace!(prefix);

        let message = msg.content.strip_prefix(prefix).unwrap_or(&msg.content);
        tracing::trace!(message);
        tracing::trace!("{:?}", self.commands.keys());
        let possible_cmd = message.split(" ").next().unwrap_or(&"Error");
        if self.commands.contains_key(possible_cmd) {
            let v = self.commands.get(possible_cmd).unwrap();
            let cctx = CommandContext {
                oshiro: Arc::clone(&ctx),
                msg: msg.clone(),
                stripped: msg.0.content.strip_prefix(&(prefix.to_owned() + possible_cmd)).unwrap_or(&msg.0.content).to_string()
            };
            (v.exec)(cctx).await?;
        }
        Ok(())
    }
}

async fn hi_echo(ctx: CommandContext) -> OshiroResult<()> {
    let a = &ctx.msg.0.author;
    ctx.oshiro
        .lock()
        .await
        .http
        .create_message(ctx.msg.channel_id)
        .content(&format!(
            "hello, {}. Here's some information about you:\n```{}```",
            a.name,
            simd_json::to_string_pretty(a)?
        ))?
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
    let oshi = ctx.oshiro.lock().await;
    let timer = Timer::new();

    let sent_msg = oshi
        .http
        .create_message(ctx.msg.channel_id)
        .content(&format!("ping!!!"))?
        .await?;

    let msg_ms = timer.elapsed_ms();

    let mut shard_latencies = vec![];

    for i in 0..oshi.shard_latency.len() {
        let avg = match oshi.shard_latency[i].average() {
            Some(x) => format!(
                "{:.3}",
                x.as_secs() as f64 / 1000.0 + f64::from(x.subsec_nanos()) * 1e-6
            ),
            None => "N/A".to_string(),
        };
        if avg != "N/A" {
            shard_latencies.push(format!(
                "shard id {:} - ws avg: {}ms - beats: {}",
                i,
                avg,
                oshi.shard_latency[i].periods()
            ))
        }
    }

    let sent = sent_msg.model().await?;

    let update = &format!("pong!\nhttp: {}ms\n{}", msg_ms, shard_latencies.join("\n"));
    let e = oshi
        .http
        .update_message(sent.channel_id, sent.id)
        .content(Some(update))?;
    e.await?;

    Ok(())
}

async fn uwu(ctx: CommandContext) -> OshiroResult<()> {
    let step1 = ctx
        .stripped
        .to_lowercase()
        .replace("l", "w")
        .replace("r", "w")
        .replace("fu", "fwu")
        .replace("na", "nya")
        .replace("ove", "uv");

    let mut dashy = true;
    let mut step2: Vec<String> = Vec::new();
    step1.split(" ").for_each(|word| {
        if word.starts_with("y") {
            if dashy == false {
                dashy = true;
                step2.push(word.to_string())
            } else {
                let yd = "y-".to_owned() + word;
                step2.push(yd);
                dashy = false;
            }
        } else {
            step2.push(word.to_string())
        };
    });

    ctx.oshiro
        .lock()
        .await
        .http
        .create_message(ctx.msg.channel_id)
        .content(&step2.join(" "))?
        .await?;
    Ok(())
}
