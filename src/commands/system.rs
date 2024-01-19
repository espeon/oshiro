use tokio::time;

use crate::{
    cmd::{CommandContext, OshiroResult},
    helper::{get_cdn_guild_asset, Timer},
    slash::{self, message},
};
use twilight_util::{builder::embed::*, snowflake::Snowflake};

use heim::{process, units, memory::memory};

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
    let platform = heim::host::platform().await?;
    let process = process::current().await.unwrap();
    let memory = memory().await.unwrap();

    // measure cpu usage
    let cpu_1 = process.cpu_usage().await.unwrap();

    time::sleep(time::Duration::from_millis(100)).await;

    let cpu_2 = process.cpu_usage().await.unwrap();

    let embed = EmbedBuilder::new()
        .title("oshiro")
        .field(
            EmbedFieldBuilder::new(
                "system",
                format!(
                    "running on {} {} ({})",
                    platform.system(),
                    platform.release(),
                    platform.hostname(),
                ),
            )
        )
        .field(
            EmbedFieldBuilder::new(
                "cpu",
                format!(
                    "{} cores, {}% usage",
                    num_cpus::get(),
                    (cpu_2 - cpu_1).get::<units::ratio::percent>().round(),
                ),
            )
        )
        .field(
            EmbedFieldBuilder::new(
                "memory",
                format!(
                    "{:.2} MB used, {:.2} GB total",
                    process.memory().await.unwrap().rss().get::<units::information::megabyte>(),
                    memory.total().get::<units::information::gigabyte>(),
                ),
            )
        )
        .field(
            EmbedFieldBuilder::new(
                "cache",
                format!(
                    "{} guilds, {} users, {} channels",
                    oshi.cache.stats().guilds(),
                    oshi.cache.stats().users(),
                    oshi.cache.stats().channels(),
                ),
            )
        )
        .image(ImageSource::url("https://i.imgur.com/V6whkQN.png")?)
        .footer(EmbedFooterBuilder::new("running on tiny horse"))
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
    //let cache_guild = oshi.cache.guild(guild_id);

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
    let mut ccount = 0;
    oshi.cache.guild_channels(guild_id).iter().for_each(|c| {
        c.iter().for_each(|_| {
            ccount += 1;
        })
    });
    let embed = embed.field(EmbedFieldBuilder::new(
        "channels",
        format!("{} (incl categories)", ccount),
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
