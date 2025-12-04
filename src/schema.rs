use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, serde::iso8601};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub id_list: String,
    pub id_members: Vec<String>,
    pub name: String,
    #[serde(with = "iso8601")]
    pub date_last_activity: OffsetDateTime,
    pub actions: Vec<Action>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub id: String,
    pub id_member_creator: String,
    #[serde(with = "iso8601")]
    pub date: OffsetDateTime,
    pub r#type: ActionType,
    pub app_creator: Option<AppCreator>,
    pub data: ActionData,
    pub member_creator: MemberCreator,
    pub limits: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum ActionType {
    UpdateCard,
    CreateCard,
    Other(String),
}

impl<'de> Deserialize<'de> for ActionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.as_str() {
            "updateCard" => Ok(ActionType::UpdateCard),
            "createCard" => Ok(ActionType::CreateCard),
            other => Ok(ActionType::Other(other.to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCreator {
    pub id: String,
    pub name: Option<String>,
    pub icon: Option<AppIcon>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppIcon {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionData {
    pub board: Board,
    pub card: CardAction,
    pub list: Option<List>,
    pub list_after: Option<List>,
    pub list_before: Option<List>,
    pub old: Option<Old>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub name: String,
    pub short_link: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardAction {
    pub id: String,
    pub id_list: Option<String>,
    pub id_short: u32,
    pub name: String,
    pub short_link: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Old {
    pub id_list: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberCreator {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub initials: String,
    pub avatar_url: Option<String>,
    pub avatar_hash: Option<String>,
    pub activity_blocked: bool,
    pub id_member_referrer: Option<String>,
    pub non_public: Option<serde_json::Value>,
    pub non_public_available: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub username: String,
    pub full_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_deserializes_the_action_type() {
        let json = r#""updateCard""#;
        let action_type: ActionType = serde_json::from_str(json).unwrap();
        assert_eq!(action_type, ActionType::UpdateCard);
    }
}
