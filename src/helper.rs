use chrono::DateTime;
use chrono::Utc;

pub struct Timer {
    start: DateTime<Utc>,
}

impl Default for Timer {
    fn default() -> Self {
        Timer { start: Utc::now() }
    }
}
impl Timer {

    pub fn new() -> Self {
        Default::default()
    }

    pub fn elapsed_ms(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.start)
            .num_milliseconds()
    }
}

pub enum UserAssetType {
    Avatar,
    DefaultAvatar,
    Banner,
}

// hash to discord cdn endpoint for user avatar
// https://cdn.discordapp.com/avatars/(userid)/(useravatar).png
pub fn hash_to_cdn_user_asset(asset_type: UserAssetType, userid: &str, useravatar: &str) -> String {
    let asset_type = match asset_type {
        UserAssetType::Avatar => "avatars",
        UserAssetType::DefaultAvatar => "embed/avatars",
        UserAssetType::Banner => "banners",
    };
    format!("https://cdn.discordapp.com/{}/{}/{}.png", asset_type, userid, useravatar)
}

pub enum GuildAssetType {
    Icon,
    Splash,
    DiscoverySplash,
    Banner,
}

// hash to discord cdn endpoint for guild icon
// https://cdn.discordapp.com/(assettype)/(guildid)/(guildicon).png
pub fn get_cdn_guild_asset(asset_type: GuildAssetType, guildid: &u64, guildicon: &str) -> String {
    let asset_type = match asset_type {
        GuildAssetType::Icon => "icons",
        GuildAssetType::Splash => "splashes",
        GuildAssetType::DiscoverySplash => "discovery-splashes",
        GuildAssetType::Banner => "banners",
    };
    format!("https://cdn.discordapp.com/{}/{}/{}.png", asset_type, guildid, guildicon)
}