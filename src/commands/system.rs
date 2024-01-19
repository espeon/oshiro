use crate::{
    cmd::{CommandContext, OshiroResult},
    helper::Timer,
    slash::{self, message},
};
use twilight_util::builder::embed::*;

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
