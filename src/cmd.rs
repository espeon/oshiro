use std::{collections::HashMap, error::Error, pin::Pin, sync::Arc};

use twilight_http::Client;

use futures::{Future};

use crate::ctx::OshiroContext;

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
        ($e: expr) => {{
            Arc::new(CommandInstance {
                name: "hi".to_string(),
                description: "hi".to_string(),
                exec: Box::new(move |ctx| Box::pin($e(ctx))),
            })
        }};
    }
}

impl CommandFramework {
    pub async fn create() -> Self {
        let mut f = CommandFramework::default();
        match f.add_command(cmd!(test_commander)).await{
            Ok(_) => println!("works"),
            Err(_) => todo!("god fuck this"),
        };
        f
    }

    pub async fn add_command(&mut self, cmd: Arc<CommandInstance>) -> OshiroResult<()> {
        self.commands.insert(
            "sal".to_string(),
            cmd,
        );
        Ok(())
    }

    pub async fn parse_command(
        &self,
        prefix: &str,
        msg: Box<twilight_model::gateway::payload::MessageCreate>,
        ctx: Arc<OshiroContext>
    ) -> OshiroResult<()> {
        let message = msg
        .content
        .strip_prefix(prefix)
        .unwrap_or(&msg.content);
        if message
            .starts_with("hello")
        {
            test_command(msg, ctx.http.clone()).await?
        } else if self.commands.contains_key(message) {
            let v = self.commands.get(message).unwrap();
            let cctx = CommandContext{
                oshiro: Arc::clone(&ctx),
                msg
            };
            (v.exec)(cctx).await?;
        }
        Ok(())
    }
}

async fn test_commander(ctx: CommandContext) -> OshiroResult<()> {
    let a = &ctx.msg.0.author.name;
    ctx.oshiro.http
        .create_message(ctx.msg.channel_id)
        .content(format!("hello {}, from Test Commander", a))?
        .await?;
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
