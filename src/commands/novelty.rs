use twilight_model::application::interaction::{
    application_command::CommandOptionValue, InteractionData,
};

use crate::{
    cmd::{CommandContext, CommandType, OshiroResult},
    slash::message,
};

pub async fn uwu(ctx: CommandContext) -> OshiroResult<()> {
    let start_string = match ctx.command_type {
        CommandType::SLASH => match ctx
            .slash
            .as_ref()
            .expect("slash command")
            .data
            .as_ref()
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
