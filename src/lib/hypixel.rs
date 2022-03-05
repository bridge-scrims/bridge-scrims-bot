//! Module to grab information from Hypixel about the API.

// #![warn(missing_docs)]

use std::{
    fmt::{self, Display},
    str::FromStr,
};

use reqwest::Client;
use serde::{Deserialize, Deserializer};

lazy_static::lazy_static! {
    /// The reqwest client for the Hypixel API
    pub(crate) static ref CLIENT: Client = Client::new();
}

type Result<T = (), E = ApiError> = std::result::Result<T, E>;

/// Hypixel API entry point
pub const ENTRY_POINT: &str = "https://api.hypixel.net";

/// Mojang API entry point
pub const MOJANG_ENTRY_POINT: &str = "https://api.mojang.com";

#[derive(Debug)]
#[non_exhaustive]
/// Error type thrown by this module
pub enum ApiError {
    /// Error from reqwest
    Http(reqwest::Error),
    /// Invalid UUID being parsed
    InvalidUUID,
    /// Authentication token to API is invalid
    NotAuthenticated,
    /// From serde
    Message(String),
    /// From serde_json
    Deser(serde_json::Error),
}

impl serde::de::Error for ApiError {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

pub fn deserialize_uuid<'de, D>(deserializer: D) -> Result<UUID, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;

    UUID::from_str(&buf).map_err(serde::de::Error::custom)
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::Deser(e)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let e = match self {
            ApiError::Http(e) => format!("http error: {}", e),
            ApiError::InvalidUUID => String::from("invalid UUID"),
            ApiError::NotAuthenticated => String::from("not authenticated"),
            ApiError::Deser(e) => format!("deserialization error: {}", e),
            ApiError::Message(m) => m.to_string(),
        };

        write!(f, "{}", e)
    }
}

impl std::error::Error for ApiError {}

/// A player UUID or API key
#[derive(Debug, PartialEq, Clone)]
pub struct UUID([u8; 16]);

impl FromStr for UUID {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.replace('-', "");
        if s.len() != 32 {
            return Err(ApiError::InvalidUUID);
        }
        Ok(Self(s.bytes().enumerate().fold(
            Ok([0u8; 16]),
            |acc, (i, c)| {
                let num = u8::from_str_radix(std::str::from_utf8(&[c]).unwrap(), 16);
                if num.is_err() {
                    return Err(ApiError::InvalidUUID);
                }
                acc.map(|mut acc| {
                    let mut num = num.unwrap();
                    if i % 2 == 0 {
                        num <<= 4;
                        acc[i / 2] = num;
                    } else {
                        acc[i / 2] += num;
                    }
                    acc
                })
            },
        )?))
    }
}

impl Display for UUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, byte) in self.0.iter().enumerate() {
            write!(f, "{:02x}", byte)?;
            if (i == 3 || i == 5 || i == 7 || i == 9) && f.alternate() {
                write!(f, "-")?;
            }
        }
        Ok(())
    }
}

/// A player, only has a UUID
pub struct Player(pub UUID);

/// Player from Mojang's API
#[derive(Deserialize)]
pub struct MojangPlayer {
    id: String,
}

impl Player {
    /// Fetches a [`Player`] from Mojang's API
    pub async fn fetch_from_username(name: String) -> Result<Self> {
        let result = CLIENT
            .get(format!(
                "{}/users/profiles/minecraft/{}",
                MOJANG_ENTRY_POINT, name
            ))
            .send()
            .await?;
        let json: MojangPlayer = result.json().await?;
        let uuid = UUID::from_str(json.id.as_str())?;
        Ok(Self(uuid))
    }
}

/// A playerdata request (GET /player)
pub struct PlayerDataRequest(pub UUID, pub Player);

// TODO: maybe use trait (if adding more requests?)
impl PlayerDataRequest {
    /// Send the request
    pub async fn send(&self) -> Result<PlayerData> {
        let response = CLIENT
            .get(format!("{}/player", ENTRY_POINT))
            .header("API-Key", self.0.to_string())
            .query(&[("uuid", self.1 .0.to_string())])
            .send()
            .await?;
        let text = response.text().await?;
        if cfg!(test) {
            eprintln!("{}", text);
        }
        let json: PlayerDataResp = serde_json::from_str(&text)?;
        Ok(json.player)
    }
}

/// Player data from Hypixel [`PlayerDataRequest`]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerData {
    /// Player's UUID
    pub uuid: String,
    /// Player's name on Hypixel
    pub displayname: Option<String>,
    /// Player's moderation rank
    pub rank: Option<PlayerRank>,
    /// Player's package rank (e.g. MVP)
    pub package_rank: Option<PackageRank>,
    /// Player's new package rank (e.g. MVP)
    pub new_package_rank: Option<PackageRank>,
    /// Player's monthly package rank (e.g. SUPERSTAR)
    pub monthly_package_rank: Option<MonthlyPackageRank>,
    // TODO: use an actual date
    /// First time on the server for the player
    pub first_login: Option<u64>,
    /// Last login time on the server for the player
    pub last_login: Option<u64>,
    /// Last logout time on the server for the player
    pub last_logout: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum PlayerRank {
    Admin,
    Moderator,
    Helper,
    Normal,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PackageRank {
    MvpPlus,
    Mvp,
    VipPlus,
    Vip,
    None,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MonthlyPackageRank {
    Superstar,
    None,
}

/// Player data response from Hypixel [`PlayerDataRequest`]
#[derive(Deserialize)]
pub struct PlayerDataResp {
    pub success: bool,
    pub player: PlayerData,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::hypixel::UUID;

    use super::*;

    #[test]
    fn uuid_parse_display() {
        let byte_array = [
            74, 223, 226, 126, 99, 211, 69, 185, 130, 56, 98, 182, 237, 111, 219, 94,
        ];
        let uuid = "4adfe27e-63d3-45b9-8238-62b6ed6fdb5e";
        let simple_uuid = "4adfe27e63d345b9823862b6ed6fdb5e";
        let parsed_uuid = UUID::from_str(uuid).unwrap();

        assert_eq!(parsed_uuid, UUID(byte_array));
        assert_eq!(format!("{}", parsed_uuid), simple_uuid);
        assert_eq!(format!("{:#}", parsed_uuid), uuid);
    }
    #[tokio::test]
    async fn notch_uuid() {
        let player = Player::fetch_from_username(String::from("Notch"))
            .await
            .unwrap();
        assert_eq!(
            player.0.to_string().as_str(),
            "069a79f444e94726a5befca90e38aaf5"
        );
    }
    #[tokio::test]
    async fn fetch_notch_info() {
        let player = Player::fetch_from_username(String::from("Briqled"))
            .await
            .unwrap();
        PlayerDataRequest(
            UUID::from_str("5c37d992-b286-468a-bedc-6a965cc3b78a").unwrap(),
            player,
        )
        .send()
        .await
        .unwrap();
    }
}
