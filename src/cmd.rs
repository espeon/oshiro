use std::{collections::HashMap, error::Error, pin::Pin, sync::Arc};

use futures::Future;
use tokio::sync::Mutex;
use twilight_model::application::interaction::Interaction;

use crate::ctx::OshiroContext;
use crate::slash::message;

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
        let uwu = crate::commands::novelty::uwu;
        let ping = crate::commands::system::ping;
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
        let guild_info = crate::commands::system::guild_info;
        cmd!(f, guild_info, "guild_info", "get info about the guild").await?;
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
            let v = match self.commands.get(possible_cmd) {
                Some(e) => e,
                None => {
                    tracing::info!("No command found for {}", possible_cmd);
                    return Ok(());
                }
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
