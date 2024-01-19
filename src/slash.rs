use std::{collections::HashMap, pin::Pin, sync::Arc};

use futures::Future;
use tokio::sync::Mutex;
use twilight_model::{
    application::{
        command::{Command, CommandOption, CommandOptionType, CommandType},
        interaction::{
            Interaction, InteractionData, InteractionType,
        },
    },
    channel::message::{Embed, MessageFlags},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::Id,
};

use crate::{
    cmd::{CommandContext, OshiroResult},
    ctx::OshiroContext,
};

pub type SlashCommandFn = Box<dyn Fn(CommandContext) -> SlashCommandResultOuter + Send + Sync>;
pub type SlashCommandResultOuter = Pin<Box<dyn Future<Output = OshiroResult> + Send>>;

pub enum CommandGroup {
    Command(SlashCommandFn),
    CommandGroup(HashMap<String, CommandGroup>),
}

pub struct CommandWrapper {
    pub command: Command,
    pub function: Option<SlashCommandFn>,
    pub subcommands: Option<HashMap<String, CommandGroup>>,
}

impl From<CommandWrapper> for Command {
    fn from(cmd_wrapper: CommandWrapper) -> Self {
        cmd_wrapper.command
    }
}

pub fn commands() -> HashMap<String, CommandWrapper> {
    let mut commands = HashMap::new();
    commands.insert(
        "ping".to_string(),
        CommandWrapper {
            command: Command {
                application_id: None,
                default_member_permissions: None,
                dm_permission: Some(true),
                description: "Get the current ping to Discord".to_owned(),
                description_localizations: None,
                guild_id: None,
                id: None,
                kind: CommandType::ChatInput,
                name: "ping".to_owned(),
                name_localizations: None,
                options: vec![],
                version: Id::new(1),
                nsfw: None,
            },
            function: Some(Box::new(move |ctx| Box::pin(crate::commands::system::ping(ctx)))),
            subcommands: None,
        },
    );
    commands.insert(
        "uwu".to_string(),
        CommandWrapper {
            command: Command {
                application_id: None,
                default_member_permissions: None,
                dm_permission: Some(true),
                description: "Uwuify a piece of text".to_owned(),
                description_localizations: None,
                guild_id: None,
                id: None,
                kind: CommandType::ChatInput,
                name: "uwu".to_owned(),
                name_localizations: None,
                options: vec![CommandOption {
                    autocomplete: None,
                    channel_types: None,
                    choices: None,
                    description: "The text you want to process".to_owned(),
                    description_localizations: None,
                    kind: CommandOptionType::String,
                    max_length: None,
                    max_value: None,
                    min_length: None,
                    min_value: None,
                    name: "text".to_owned(),
                    name_localizations: None,
                    options: None,
                    required: Some(true),
                }],
                version: Id::new(1),
                nsfw: None,
            },
            function: Some(Box::new(move |ctx| {
                Box::pin(crate::commands::novelty::uwu(ctx))
            })),
            subcommands: None,
        },
    );
    commands.insert(
        "stats".to_string(),
        CommandWrapper {
            command: Command {
                application_id: None,
                default_member_permissions: None,
                dm_permission: Some(true),
                description: "Server statistics".to_owned(),
                description_localizations: None,
                guild_id: None,
                id: None,
                kind: CommandType::ChatInput,
                name: "stats".to_owned(),
                name_localizations: None,
                options: vec![],
                version: Id::new(1),
                nsfw: None,
            },
            function: Some(Box::new(move |ctx| {
                Box::pin(crate::commands::system::stats(ctx))
            })),
            subcommands: None,
        },
    );
    commands.insert(
        "server".to_string(),
        CommandWrapper {
            command: Command {
                application_id: None,
                default_member_permissions: None,
                dm_permission: Some(true),
                description: "Server statistics".to_owned(),
                description_localizations: None,
                guild_id: None,
                id: None,
                kind: CommandType::ChatInput,
                name: "server".to_owned(),
                name_localizations: None,
                options: vec![CommandOption {
                    name: "info".to_string(),
                    options: None,
                    autocomplete: None,
                    channel_types: None,
                    choices: None,
                    description: "Get info about the server".to_string(),
                    description_localizations: None,
                    kind: CommandOptionType::SubCommand,
                    max_length: None,
                    max_value: None,
                    min_length: None,
                    min_value: None,
                    name_localizations: None,
                    required: None,
                }],
                version: Id::new(1),
                nsfw: None,
            },
            function: None,
            subcommands: Some({
                let mut subcommands: HashMap<String, CommandGroup> = HashMap::new();
                subcommands.insert(
                    "info".to_string(),
                    CommandGroup::Command(Box::new(move |ctx| Box::pin(crate::commands::system::guild_info(ctx)))),
                );
                subcommands
            }),
        },
    );
    commands
}

pub async fn handle(slash: Interaction, ctx: Arc<Mutex<OshiroContext>>) -> OshiroResult<()> {
    let slash = match slash.kind {
        InteractionType::Ping => {
            tracing::warn!("Got a ping!");
            return Ok(());
        }
        InteractionType::ApplicationCommand => Some(slash),
        InteractionType::MessageComponent => None,
        InteractionType::ApplicationCommandAutocomplete => {
            None
        }
        InteractionType::ModalSubmit => None,
        _ => None,
    };
    //works _=ctx.lock().await.interaction().create_response(slash.id, &slash.token, &message("hello")).await?;
    let some_slash = slash.clone().unwrap();
    let data = if let Some(InteractionData::ApplicationCommand(data)) = some_slash.data.clone() {
        data
    } else {
        return Err("No application data".into());
    };
    let name = data.name.as_str();

    // building the whole command name

    let mut alloptions = vec![name];

    data.options.iter().for_each(|o| {
        alloptions.push(o.name.as_str());
        // if o.value is not a CommandOptionValue::SubCommand or SubCommandGroup
        match &o.value {
            twilight_model::application::interaction::application_command::CommandOptionValue::SubCommand(s) => {
                s.iter().for_each(|o| {
                    alloptions.push(o.name.as_str());
                })
            }
            twilight_model::application::interaction::application_command::CommandOptionValue::SubCommandGroup(s) => {
                s.iter().for_each(|o| {
                    alloptions.push(o.name.as_str());
                    if let twilight_model::application::interaction::application_command::CommandOptionValue::SubCommand(s) = &o.value {
                        s.iter().for_each(|o| {
                            alloptions.push(o.name.as_str());
                        })
                    }
                })
            },
            _ => {}
        }
    });

    let fname = alloptions.join(" ");

    // creating CommandContext
    let cctx = CommandContext {
        command_type: crate::cmd::CommandType::SLASH,
        oshiro: Arc::clone(&ctx),
        msg: None,
        stripped: None,
        slash: slash.clone(),
    };

    tracing::info!("Slash command used: {}", fname);
    let c = commands();
    let fun = match c.get(name) {
        Some(c) => {
            // check if command name provided is a subcommand
            if alloptions.len() > 1 {
                let cmd = alloptions[1];
                // check if subcommand exists
                match c.subcommands {
                    Some(ref subcommands) => {
                        // check if subcommand is a subcommand
                        match subcommands.get(cmd) {
                            // if it does, run it
                            Some(CommandGroup::Command(f)) => {
                                f
                            }
                            Some(CommandGroup::CommandGroup(g)) => {
                                // check if subcommand is a subcommand group
                                if alloptions.len() > 2 {
                                    let cmd = alloptions[2];
                                    // check if subcommand group exists
                                    match g.get(cmd) {
                                        // if it does, run it
                                        Some(CommandGroup::Command(f)) => {
                                            f
                                        }
                                        _ => {
                                            tracing::warn!("Error when running a command inside a subcommand group {:?}", slash);
                                            return Ok(());
                                        }
                                    }
                                } else {
                                    tracing::warn!("Error: Command vector is not 3 units long {:?}", slash);
                                    return Ok(());
                                }
                            }
                            None => {
                                tracing::warn!("Could not find the command inside a subcommand {:?}", slash);
                                return Ok(());
                            }
                        }
                    }
                    None => {
                        tracing::warn!("No subcommand found {:?}", slash);
                        return Ok(());
                    }
                }
            } else {
                match &c.function {
                    Some(f) => {
                        f
                    }
                    None => {
                        tracing::warn!("Unhandled command! {:?}", slash);
                        return Ok(());
                    }
                }
            }
            
        }
        None => {
            tracing::warn!("Unhandled command! {:?}", slash);
            return Ok(());
        }
    };

    // run the command
    match (fun)(cctx).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error when running a command {:?}", e);
            // report the command to the user
            let resp = error(&format!("Error when running a command {:?}", e));
            if let Some(slash) = slash {
                ctx.lock().await.interaction().create_response(slash.id, &slash.token, &resp).await?;
            }
        }
    }

    Ok(())
}

pub fn error(msg: &str) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            allowed_mentions: None,
            attachments: None,
            choices: None,
            components: None,
            content: Some(msg.to_owned()),
            custom_id: None,
            embeds: None,
            flags: Some(MessageFlags::EPHEMERAL),
            title: None,
            tts: None,
        }),
    }
}

pub fn message(msg: &str) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            allowed_mentions: None,
            attachments: None,
            choices: None,
            components: None,
            content: Some(msg.to_owned()),
            custom_id: None,
            embeds: None,
            flags: None,
            title: None,
            tts: None,
        }),
    }
}

pub fn embed(embeds: Vec<Embed>) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            allowed_mentions: None,
            attachments: None,
            choices: None,
            components: None,
            content: None,
            custom_id: None,
            embeds: Some(embeds),
            flags: None,
            title: None,
            tts: None,
        }),
    }
}

pub fn ephemeral_message(msg: &str) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            allowed_mentions: None,
            attachments: None,
            choices: None,
            components: None,
            content: Some(msg.to_owned()),
            custom_id: None,
            embeds: None,
            flags: Some(MessageFlags::EPHEMERAL),
            title: None,
            tts: None,
        }),
    }
}
