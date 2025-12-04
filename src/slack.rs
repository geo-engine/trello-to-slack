use crate::{SlackUser, config::SlackConfig};
use anyhow::{Result, bail};

pub struct SlackMessagePoster {
    client: reqwest::blocking::Client,
    bot_token: String,
}

impl SlackMessagePoster {
    pub fn new(client: reqwest::blocking::Client, config: &SlackConfig) -> Self {
        SlackMessagePoster {
            client,
            bot_token: config.bot_token.clone(),
        }
    }

    pub fn post_message(&self, slack_user: &SlackUser, message: &str) -> Result<()> {
        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&serde_json::json!({
                "channel": slack_user.0,
                "markdown_text": message
            }))
            .send()?;

        if !response.status().is_success() {
            bail!("Failed to send message: {:?}", response.text()?);
        }

        Ok(())
    }
}
