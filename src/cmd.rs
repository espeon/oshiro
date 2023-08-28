use std::{collections::HashMap, error::Error, pin::Pin, sync::Arc};

use futures::Future;
use tokio::sync::Mutex;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::InteractionData;

use crate::ctx::OshiroContext;
use crate::slash::message;

use chrono::DateTime;
use chrono::Utc;

pub type OshiroResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

pub type CommandFn = Box<dyn Fn(CommandContext) -> CommandResultOuter + Send + Sync>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = OshiroResult> + Send>>;

/// Describes a command
pub struct CommandInstance {
    pub name: String,
    pub description: String,
    pub exec: CommandFn,
}

#[derive(PartialEq)]
pub enum CommandType {
    SLASH,
    TEXT,
}

/// A struct passed to the command when launched
pub struct CommandContext {
    pub command_type: CommandType,
    pub oshiro: Arc<Mutex<OshiroContext>>,
    pub msg: Option<Box<twilight_model::channel::Message>>,
    pub stripped: Option<String>,
    pub slash: Option<Interaction>,
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
        cmd!(f, uwu, "uwu", "uwuifies strings xD").await?;
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
        let possible_cmd = message.split(' ').next().unwrap_or("No command specified");
        if self.commands.contains_key(possible_cmd) {
            let v = match self.commands.get(possible_cmd){
                Some(e) => e,
                None => {tracing::info!("No command found for {}", possible_cmd); return Ok(())},
            };
            let cctx = CommandContext {
                command_type: CommandType::TEXT,
                oshiro: Arc::clone(&ctx),
                msg: Some(Box::new(msg.0.clone())),
                stripped: Some(
                    msg.content
                        .strip_prefix(&(prefix.to_owned() + possible_cmd))
                        .unwrap_or(&msg.0.content)
                        .to_string(),
                ),
                slash: None,
            };
            (v.exec)(cctx).await?;
        }
        Ok(())
    }
}

async fn hi_echo(ctx: CommandContext) -> OshiroResult<()> {
    if ctx.command_type == CommandType::TEXT {
        let msg = ctx.msg.expect("is text");

        let a = msg.author;
        ctx.oshiro
            .lock()
            .await
            .http
            .create_message(msg.channel_id)
            .content(&format!(
                "hello, {}. Here's some information about you:\n```{}```",
                a.name,
                simd_json::to_string_pretty(&a)?
            ))?
            .await?;
    } else {
        let slash = ctx.slash.expect("is slash command");
        let user = slash.user.expect("slash command has user");
        ctx.oshiro
            .lock()
            .await
            .interaction()
            .create_response(
                slash.id,
                &slash.token,
                &message(&format!(
                    "hello, {}. Here's some information about you:\n```{}```",
                    user.name,
                    simd_json::to_string_pretty(&user)?
                )),
            )
            .await?;
    }
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
pub async fn ping(ctx: CommandContext) -> OshiroResult<()> {
    let oshi = ctx.oshiro.lock().await;
    let timer = Timer::new();

    // send first message
    if let Some(slash) = &ctx.slash {
        oshi.interaction()
            .create_response(slash.id, &slash.token, &message("eyup"))
            .await?;
    }
    // text command - we don't need a response with slash commands as
    // we can update via the slash command's token instead
    let sent_msg = if let Some(msg) = ctx.msg {
        Some(
            oshi.http
                .create_message(msg.channel_id)
                .content("ping!!!")?
                .await?,
        )
    } else {
        None
    };

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

    let update = format!("pong!\nhttp: {}ms\n{}", msg_ms, shard_latencies.join("\n"));

    if let Some(sent_msg) = sent_msg {
        // if sent via text command
        let sent = sent_msg.model().await?;

        let e = oshi
            .http
            .update_message(sent.channel_id, sent.id)
            .content(Some(&update))?;
        e.await?;
    } else if let Some(slash) = ctx.slash {
        // if sent via slash command
        oshi.interaction()
            .update_response(&slash.token)
            .content(Some(&update))?
            .await?;
    }

    Ok(())
}

pub async fn uwu(ctx: CommandContext) -> OshiroResult<()> {
    let start_string = match ctx.command_type {
        CommandType::SLASH => match ctx
            .slash.as_ref()
            .expect("slash command")
            .data.as_ref()
            .expect("Application command data")
        {
            InteractionData::ApplicationCommand(e) => {
                if let CommandOptionValue::String(out) =
                    &e.options.first().expect("Has text parameter").value
                {
                    out.to_owned()
                } else {
                    "Something broke!".to_owned()
                }
            }
            _ => todo!(),
        },
        CommandType::TEXT => ctx.stripped.expect("text command"),
    };

    // multiple string replacements
    // str::replace replaces every instance
    let step1 = start_string
        .to_lowercase()
        .replace(['l', 'r', 'v'], "w")
        .replace("fu", "fwu")
        .replace("na", "nya")
        .replace("ove", "uv");


    // add a "y-" to the beginning of a word every *other* time 
    // a "y" shows up at the beginning of a word
    let mut dashy = true;
    let mut step2: Vec<String> = Vec::new();
    step1.split(' ').for_each(|word| {
        if word.starts_with('y') {
            if !dashy {
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

    match ctx.command_type {
        CommandType::SLASH => {
            let oshi = ctx.oshiro.lock().await;
            if let Some(slash) = &ctx.slash {
                oshi.interaction()
                    .create_response(slash.id, &slash.token, &message(&step2.join(" ")))
                    .await?;
            };
        }
        CommandType::TEXT => {
            ctx.oshiro
                .lock()
                .await
                .http
                .create_message(ctx.msg.expect("text command").channel_id)
                .content(&step2.join(" "))?
                .await?;
        }
    };
    Ok(())
}
