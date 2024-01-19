use crate::{
    cmd::{CommandContext, OshiroResult},
    helper::{get_cdn_guild_asset, Timer},
    slash::{self, message},
};
use twilight_model::user::User;
use twilight_util::{builder::embed::*, snowflake::Snowflake};

pub async fn ping(ctx: CommandContext) -> OshiroResult<()> {
    let oshi = ctx.oshiro.lock().await;
    let timer = Timer::new();

    // send first message
    if let Some(slash) = &ctx.slash {
        oshi.interaction()
            .create_response(slash.id, &slash.token, &message("ayup"))
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

pub async fn stats(ctx: CommandContext) -> OshiroResult {
    let oshi = ctx.oshiro.lock().await;
    let embed = EmbedBuilder::new()
        .title("running on tiny horse")
        .image(ImageSource::url("https://i.imgur.com/V6whkQN.png")?)
        .validate()?
        .build();
    let embeds = vec![embed];
    match ctx.command_type {
        crate::cmd::CommandType::SLASH => {
            let resp = slash::embed(embeds);
            let slash = ctx.slash.expect("slash command");
            oshi.interaction()
                .create_response(slash.id, &slash.token, &resp)
                .await?;
        }
        crate::cmd::CommandType::TEXT => {
            oshi.http
                .create_message(ctx.msg.expect("text channel message object").channel_id)
                .embeds(&embeds)?
                .await?;
        }
    }
    Ok(())
}

pub async fn guild_info(ctx: CommandContext) -> OshiroResult {
    let oshi = ctx.oshiro.lock().await;

    // get the guild id
    let guild_id = match &ctx.slash {
        Some(slash) => slash.guild_id.expect("slash command has guild id"),
        None => {
            // if it's a text command, get the guild id from the message
            let msg = ctx.msg.clone().expect("text command has message");
            msg.guild_id.expect("text command has guild id")
        }
    };

    // get guild through http
    let guild = oshi
        .http
        .guild(guild_id)
        .with_counts(true)
        .await
        .expect("guild")
        .model()
        .await?;
    // get cache guild object
    let cache_guild = oshi.cache.guild(guild_id);

    // set up the embed
    let embed = EmbedBuilder::new();

    // name of guild
    let embed = embed.title(guild.name);

    // icon of guild
    let embed = if let Some(icon) = guild.icon {
        embed.thumbnail(ImageSource::url(get_cdn_guild_asset(
            crate::helper::GuildAssetType::Icon,
            &guild_id.id(),
            &icon.to_string(),
        ))?)
    } else {
        embed
    };

    // description of guild
    let embed = if let Some(description) = guild.description {
        embed.description(description)
    } else {
        embed
    };

    // owner of guild
    let owner = oshi
        .http
        .user(guild.owner_id)
        .await
        .expect("owner")
        .model()
        .await?;
    let owner_discrim = if owner.discriminator == 0000 {
        "".to_string()
    } else {
        format!("#{}", owner.discriminator)
    };

    let embed = embed.field(EmbedFieldBuilder::new(
        "owner",
        format!("{}{} (<@{}>)", owner.name, owner_discrim, owner.id),
    ));

    dbg!(oshi.cache.guild_channels(guild_id));

    // channel count
    let embed = embed.field(EmbedFieldBuilder::new(
        "channels",
        match oshi.cache.guild_channels(guild_id) {
            Some(channels) => {
                let mut channel_count = 0;
                for _ in channels.iter() {
                    channel_count += 1;
                }
                format!("{}", channel_count)
            }
            None => "could not fetch".to_string(),
        },
    ));

    // member count
    let embed = embed.field(EmbedFieldBuilder::new(
        "members",
        match guild.approximate_member_count {
            Some(count) => format!("{}", count),
            None => "could not fetch".to_string(),
        },
    ));

    // emoji and sticker count
    let embed = embed.field(EmbedFieldBuilder::new(
        "emojis and stickers",
        format!(
            "{} emojis\n{} stickers",
            guild.emojis.len(),
            guild.stickers.len()
        ),
    ));

    // features
    let disp: Vec<String> = guild.features.iter().map(|f| format!("{:?}", f)).collect();
    let embed = embed.field(EmbedFieldBuilder::new(
        "features",
        format!("```{}```",disp.join("\n")),
    ));

    // splash of guild
    let embed = if let Some(splash) = guild.splash {
        embed.image(ImageSource::url(get_cdn_guild_asset(
            crate::helper::GuildAssetType::Splash,
            &guild_id.id(),
            &splash.to_string(),
        ))?)
    } else {
        embed
    };

    // banner of guild
    let embed = if let Some(banner) = guild.banner {
        embed.image(ImageSource::url(get_cdn_guild_asset(
            crate::helper::GuildAssetType::Banner,
            &guild_id.id(),
            &banner.to_string(),
        ))?)
    } else {
        embed
    };

    let embeds = vec![embed.validate()?.build()];
    match ctx.command_type {
        crate::cmd::CommandType::SLASH => {
            let resp = slash::embed(embeds);
            let slash = ctx.slash.expect("slash command");
            oshi.interaction()
                .create_response(slash.id, &slash.token, &resp)
                .await?;
        }
        crate::cmd::CommandType::TEXT => {
            oshi.http
                .create_message(ctx.msg.expect("text channel message object").channel_id)
                .embeds(&embeds)?
                .await?;
        }
    }
    Ok(())
}
