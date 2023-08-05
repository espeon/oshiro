use std::sync::Arc;

use tokio::sync::Mutex;
use twilight_model::{
    application::{
        command::{Command, CommandOption, CommandType, CommandOptionType},
        interaction::{Interaction, InteractionData, InteractionType},
    },
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::Id,
};

use crate::{
    cmd::{CommandContext, OshiroResult},
    ctx::OshiroContext,
};

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            application_id: None,
            default_member_permissions: None,
            dm_permission: Some(true),
            description: "Ping times".to_owned(),
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
        Command {
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
                required: Some(true)
            }],
            version: Id::new(1),
            nsfw: None,
        },
    ]
}

pub async fn handle(slash: Interaction, ctx: Arc<Mutex<OshiroContext>>) -> OshiroResult<()> {
    let slash = match slash.kind {
        InteractionType::Ping => {
            tracing::warn!("Got slash ping!");
            return Ok(());
        }
        InteractionType::ApplicationCommand => slash,
        InteractionType::MessageComponent => todo!(),
        InteractionType::ApplicationCommandAutocomplete => {
            todo!();
        }
        InteractionType::ModalSubmit => todo!(),
        _ => todo!(),
    };
    //works _=ctx.lock().await.interaction().create_response(slash.id, &slash.token, &message("hello")).await?;
    let data = if let Some(InteractionData::ApplicationCommand(data)) = slash.data.clone() {
        data
    } else {
        return Err("No application data".into());
    };
    let name = data.name.as_str();

    // creating CommandContext
    let cctx = CommandContext {
        command_type: crate::cmd::CommandType::SLASH,
        oshiro: Arc::clone(&ctx),
        msg: None,
        stripped: None,
        slash: Some(slash.clone()),
    };

    tracing::info!("Slash command used: {}", name);
    // TODO: replace this with a better type of command matching system
    match name {
        "ping" => {
            tracing::info!("ping");
            crate::cmd::ping(cctx).await
        }
        "uwu" => {
            tracing::info!("uwu");
            crate::cmd::uwu(cctx).await
        }
        _ => {
            tracing::warn!("Unhandled command! {:?}", slash);
            return Ok(());
        }
    }?;
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
