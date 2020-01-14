use crate::{Tarkov, Result, GAME_VERSION, UNITY_VERSION, PROD_ENDPOINT, Error};
use serde::{Deserialize, Serialize, de};
use flate2::read::ZlibDecoder;
use std::io::Read;
use actix_web::http::StatusCode;
use std::fmt::Write;
use core::fmt;
use serde::de::Unexpected;

#[derive(Debug, Deserialize)]
struct ProfileResponse {
    #[serde(rename = "err")]
    error_code: u8,
    #[serde(rename = "errmsg")]
    error_message: Option<String>,
    data: Option<Vec<ProfileData>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProfileInfo {
    nickname: String,
    // XXX: BAD DEVS!
    lower_nickname: Option<String>,
    lowercase_nickname: Option<String>,
    // TODO: This can be enum
    side: String,
    voice: String,
    level: u64,
    experience: u64,
    registration_date: u64,
    game_version: String,
    account_type: u64,
    // XXX: Bad devs! This field can be both String and integer, ignoring for now.
    // member_category: String,
    #[serde(rename = "lockedMoveCommands")]
    locked_move_commands: bool,
    savage_lock_time: u64,
    last_time_played_as_savage: u64,
    settings: ProfileInfoSettings,
    need_wipe: bool,
    global_wipe: bool,
    nickname_change_date: u64,
    // bans: [] TODO: Type unknown
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProfileInfoSettings {
    role: Option<String>,
    bot_difficulty: Option<String>,
    experience: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProfileCustomization {
    head: String,
    body: String,
    feet: String,
    hands: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Health {
    current: u64,
    maximum: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BodyParts {
    head: Head,
    chest: Chest,
    stomach: Stomach,
    left_arm: LeftArm,
    right_arm: RightArm,
    left_leg: LeftLeg,
    right_leg: RightLeg,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Head {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Chest {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Stomach {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeftArm {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RightArm {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeftLeg {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RightLeg {
    health: Health
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProfileHealth {
    hydration: Health,
    energy: Health,
    body_parts: BodyParts,
    update_time: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Upd {
    stack_objects_count: Option<u64>,
    spawned_in_session: Option<bool>,
    med_kit: Option<UpdMedkit>,
    repairable: Option<UpdRepairable>,
    light: Option<UpdLight>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdMedkit {
    hp_resource: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdRepairable {
    max_durability: u64,
    durability: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdLight {
    is_active: bool,
    selected_mode: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    x: u64,
    y: u64,
    r: u64,
    is_searched: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_tpl")]
    tpl: String,
    parent_id: Option<String>,
    slot_id: Option<String>,
//    location: Option<Location>, TODO: Bad type...
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInventory {
    items: Vec<Item>,
    equipment: String,
    stash: Option<String>,
    quest_raid_items: String,
    quest_stash_items: String,
    // first_panel: {} // TODO: Type unknown
}

#[derive(Debug, Deserialize)]
pub struct ProfileData {
    #[serde(rename = "_id")]
    id: String,
    aid: u64,
    savage: Option<String>,
    #[serde(rename = "Info")]
    info: ProfileInfo,
    #[serde(rename = "Customization")]
    customization: ProfileCustomization,
    #[serde(rename = "Health")]
    health: ProfileHealth,
    #[serde(rename = "Inventory")]
    inventory: ProfileInventory,
}

#[derive(Debug, err_derive::Error)]
pub enum ProfileError {
    #[error(display = "not authorized or game profile not selected")]
    NotAuthorized,
}

#[derive(Debug, Serialize)]
struct SelectRequest<'a> {
    uid: &'a str,
}

#[derive(Deserialize)]
struct SelectResponse {
    #[serde(rename = "err")]
    error_code: u8,
    #[serde(rename = "errmsg")]
    error_message: Option<String>,
    status: Option<String>,
}

#[derive(Debug, err_derive::Error)]
pub enum SelectError {
    #[error(display = "invalid user id selected")]
    InvalidUserID,
}

impl Tarkov {
    pub async fn get_profiles(&self) -> Result<Vec<ProfileData>> {
        let url = format!("{}/client/game/profile/list", PROD_ENDPOINT);
        let mut res = self.client
            .post(url)
            .header("User-Agent", format!("UnityPlayer/{} (UnityWebRequest/1.0, libcurl/7.52.0-DEV)", UNITY_VERSION))
            .header("App-Version", format!("EFT Client {}", GAME_VERSION))
            .header("X-Unity-Version", UNITY_VERSION)
            .header("Cookie", format!("PHPSESSID={}", self.session))
            .send_json(&{})
            .await?;

        let body = res.body().await?;
        let mut decode = ZlibDecoder::new(&body[..]);
        let mut body = String::new();
        decode.read_to_string(&mut body)?;

        match res.status() {
            StatusCode::OK => {
                let res = serde_json::from_slice::<ProfileResponse>(body.as_bytes())?;
                match res.error_code {
                    0 => Ok(res
                        .data
                        .expect("API returned no errors but `data` is unavailable.")),
                    201 => Err(ProfileError::NotAuthorized)?,
                    _ => Err(Error::UnknownAPIError(res.error_code)),
                }
            }
            _ => Err(Error::Status(res.status())),
        }
    }

    pub async fn select_profile(&self, user_id: &str) -> Result<()> {
        let url = format!("{}/client/game/profile/select", PROD_ENDPOINT);
        let mut res = self.client
            .post(url)
            .header("User-Agent", format!("UnityPlayer/{} (UnityWebRequest/1.0, libcurl/7.52.0-DEV)", UNITY_VERSION))
            .header("App-Version", format!("EFT Client {}", GAME_VERSION))
            .header("X-Unity-Version", UNITY_VERSION)
            .header("Cookie", format!("PHPSESSID={}", self.session))
            .send_json(&SelectRequest { uid: user_id })
            .await?;

        let body = res.body().await?;
        let mut decode = ZlibDecoder::new(&body[..]);
        let mut body = String::new();
        decode.read_to_string(&mut body)?;

        match res.status() {
            StatusCode::OK => {
                let res = serde_json::from_slice::<SelectResponse>(body.as_bytes())?;
                match res.error_code {
                    0 => Ok(()),
                    205 => Err(SelectError::InvalidUserID)?,
                    _ => Err(Error::UnknownAPIError(res.error_code)),
                }
            }
            _ => Err(Error::Status(res.status())),
        }
    }
}

// XXX: I shouldn't have to do this if tarkov devs know what types are.
struct LocationVisitor;

impl<'de> de::Visitor<'de> for LocationVisitor {
    type Value = Option<Location>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Location struct")
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, d: D) -> std::result::Result<Self::Value, D::Error>
        where
            D: de::Deserializer<'de>,
    {
        Ok(None)
    }
}

pub fn deserialize_location_or_none<'de, D>(d: D) -> std::result::Result<Option<Location>, D::Error>
    where
        D: de::Deserializer<'de>,
{
    d.deserialize_option(LocationVisitor)
}