use crate::{SlackUser, TrelloUser};
use clap::{Args, Parser};

#[derive(Clone, Debug, Parser)]
pub struct AppConfig {
    /// Action to perform
    #[command(subcommand)]
    pub action: ActionConfig,

    #[command(flatten)]
    pub slack: SlackConfig,
    #[command(flatten)]
    pub trello: TrelloConfig,
    /// Maps Trello users to Slack users
    #[arg(long, num_args=1.., value_delimiter = ',', value_parser=parse_user_mapping, env="USER_MAPPING")]
    pub user_mapping: Vec<UserMapping>,
}

#[derive(Clone, Debug, Parser)]
pub enum ActionConfig {
    /// Send messages for cards that are pending reviews
    PendingReviews,
    /// Send notifications for inactive cards
    InactiveCards,
}

impl std::fmt::Display for ActionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionConfig::PendingReviews => write!(f, "PendingReviews"),
            ActionConfig::InactiveCards => write!(f, "InactiveCards"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserMapping {
    pub trello_user: TrelloUser,
    pub slack_user: SlackUser,
}

fn parse_user_mapping(s: &str) -> Result<UserMapping, String> {
    let parts: Vec<&str> = s.split('=').map(str::trim).collect();
    if parts.len() != 2 {
        return Err(format!("Invalid user mapping format: {s}"));
    }
    Ok(UserMapping {
        trello_user: TrelloUser(parts[0].to_string()),
        slack_user: SlackUser(parts[1].to_string()),
    })
}

#[derive(Clone, Debug, Args)]
pub struct SlackConfig {
    #[arg(long = "slack-bot-token", env = "SLACK_BOT_TOKEN")]
    pub bot_token: String,
}

#[derive(Clone, Debug, Args)]
pub struct TrelloConfig {
    #[arg(long = "trello-key", env = "TRELLO_KEY")]
    pub key: String,
    #[arg(long = "trello-token", env = "TRELLO_TOKEN")]
    pub token: String,

    /// Boards to gather members from
    #[arg(
        long = "trello-board-ids",
        env = "TRELLO_BOARD_IDS",
        num_args=1..,
        value_delimiter = ','
    )]
    pub board_ids: Vec<String>,

    /// Lists to consider for review requests
    #[arg(
        long = "trello-review-lists",
        env = "TRELLO_REVIEW_LISTS",
        num_args=1..,
        value_delimiter = ','
    )]
    pub review_lists: Vec<String>,

    /// Lists to consider for inactive cards
    #[arg(
        long = "trello-inactive-cards-lists",
        env = "TRELLO_INACTIVE_CARDS_LISTS",
        num_args=1..,
        value_delimiter = ','
    )]
    pub inactive_cards_lists: Vec<String>,
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_load_config() {
        // let config_content = r#"
        // slack_token = "xoxb-slack-token"
        // trello_key = "trello-key"
        // trello_token = "trello-token"
        // [user_mapping]
        // "trello_user1" = "slack_user1"
        // "trello_user2" = "slack_user2"
        // "#;

        // let config_path = "test_config.toml";
        // let mut file = File::create(config_path).unwrap();
        // file.write_all(config_content.as_bytes()).unwrap();

        // let config = AppConfig::new(config_path).unwrap();
        // assert_eq!(config.slack_token, "xoxb-slack-token");
        // assert_eq!(config.trello_key, "trello-key");
        // assert_eq!(config.trello_token, "trello-token");
        // assert_eq!(
        //     config.user_mapping.get("trello_user1").unwrap(),
        //     "slack_user1"
        // );

        // std::fs::remove_file(config_path).unwrap();
    }
}
