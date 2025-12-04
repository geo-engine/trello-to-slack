use crate::{
    config::TrelloConfig,
    schema::{ActionType, Card, List, Member},
    util::{debug_write_to_file, is_sorted_descending},
};
use anyhow::{Context, Result, bail};
use reqwest::header::ACCEPT;

pub struct TrelloClient {
    client: reqwest::Client,
    key: String,
    token: String,
}

impl TrelloClient {
    pub fn new(client: reqwest::Client, config: &TrelloConfig) -> Self {
        TrelloClient {
            client,
            key: config.key.clone(),
            token: config.token.clone(),
        }
    }

    pub async fn get_members(&self, board_id: &str) -> Result<Vec<Member>> {
        let response = self
            .client
            .get(format!(
                "https://api.trello.com/1/boards/{board_id}/members"
            ))
            .query(&[("key", &self.key), ("token", &self.token)])
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to send message: {:?}", response.text().await?);
        }

        let json: serde_json::Value = response.json().await?;

        debug_write_to_file(&json, &format!("debug/members_{board_id}.json"), "Members")?;

        let members: Vec<Member> =
            serde_json::from_value(json).context("Could not parse JSON response")?;
        Ok(members)
    }

    pub async fn get_lists(&self, board_id: &str) -> Result<Vec<List>> {
        let response = self
            .client
            .get(format!("https://api.trello.com/1/boards/{board_id}/lists"))
            .query(&[
                ("key", self.key.as_ref()),
                ("token", self.token.as_ref()),
                ("cards", "none"),
                ("fields", "id,name"),
            ])
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to send message: {:?}", response.text().await?);
        }

        let json: serde_json::Value = response.json().await?;

        debug_write_to_file(&json, &format!("debug/lists_{board_id}.json"), "Boards")?;

        let lists: Vec<List> =
            serde_json::from_value(json).context("Could not parse JSON response")?;
        Ok(lists)
    }

    pub async fn get_cards(&self, list_id: &str) -> Result<Vec<Card>> {
        let response = self
            .client
            .get(format!("https://api.trello.com/1/lists/{list_id}/cards"))
            .query(&[
                ("key", self.key.as_ref()),
                ("token", self.token.as_ref()),
                ("fields", "name,idList,idMembers,dateLastActivity,url"),
                ("actions", "updateCard:idList,createCard"),
            ])
            .header(ACCEPT, "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Failed to send message: {:?}", response.text().await?);
        }

        let json: serde_json::Value = response.json().await?;

        debug_write_to_file(&json, &format!("debug/cards_{list_id}.json"), "Cards")?;

        let mut cards: Vec<Card> =
            serde_json::from_value(json).context("Could not parse JSON response")?;
        cards.sort_by_key(|card| card.actions.first().map(|action| action.date));
        Ok(cards)
    }
}

pub fn last_update_from_card(card: &Card) -> time::OffsetDateTime {
    card.date_last_activity
}

pub fn moved_to_list_date(card: &Card) -> Result<time::OffsetDateTime> {
    debug_assert!(
        is_sorted_descending(&card.actions),
        "Card actions are not sorted descending by date"
    );

    // Actions are returned newest first. We look for the MOST RECENT move INTO this list.
    for action in &card.actions {
        match action.r#type {
            // A: Card was moved INTO the current list
            ActionType::UpdateCard => {
                if let Some(list_after) = &action.data.list_after
                    && list_after.id == card.id_list
                {
                    return Ok(action.date);
                }
            }
            // B: Card was created in the current list (and never moved)
            ActionType::CreateCard => {
                if action.data.card.id_list.as_deref() == Some(&card.id_list) {
                    return Ok(action.date);
                }
            }

            ActionType::Other(_) => {}
        }
    }

    // Fallback: If no relevant action was found (e.g., deleted history or edge case), derive creation date from the card's ID
    creation_date_from_card_id(&card.id)
}

/// cf. <https://support.atlassian.com/trello/docs/getting-the-time-a-card-or-board-was-created/>
pub fn creation_date_from_card_id(card_id: &str) -> Result<time::OffsetDateTime> {
    if card_id.len() < 8 {
        bail!("Card ID is too short to extract timestamp");
    }

    let timestamp_hex = &card_id[..8];
    let timestamp_int = u32::from_str_radix(timestamp_hex, 16)?;
    let unix_timestamp = time::OffsetDateTime::from_unix_timestamp(i64::from(timestamp_int))?;

    Ok(unix_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{OffsetDateTime, macros::datetime};

    #[test]
    fn it_extracts_creation_date_from_card_id() {
        let date = creation_date_from_card_id("4d5ea62fd76aa1136000000c").unwrap();

        assert_eq!(
            date,
            OffsetDateTime::from_unix_timestamp(1_298_048_559).unwrap()
        );
    }

    #[test]
    fn it_finds_last_update_from_card_actions() {
        let json = serde_json::json!({
          "id": "68ef38d7dea64db678b21e50",
          "idList": "5fce1e1ebb7b5d587c848801",
          "idMembers": [
            "5fc6420fa93cf1309db65b09"
          ],
          "name": "Prüfen, ob alle E-Mails an weitergeleitet wurden.",
          "dateLastActivity": "+002025-11-21T11:50:59.295000000Z",
          "url": "https://trello.com/c/WtgfKH5P/2875-prüfen-ob-alle-e-mails-weitergeleitet-wurden",
          "actions": [
            {
              "id": "68ff67ccf6804d2f8e7c5ade",
              "idMemberCreator": "5fc5fc01b18f8769073220f0",
              "date": "+002025-10-27T12:38:36.472000000Z",
              "type": "updateCard",
              "appCreator": null,
              "data": {
                "board": {
                  "id": "5fce1e1ebb7b5d587c8487ff",
                  "name": "Management",
                  "shortLink": "NWwUCtTl"
                },
                "card": {
                  "id": "68ef38d7dea64db678b21e50",
                  "idList": "5fce1e1ebb7b5d587c848801",
                  "idShort": 2875,
                  "name": "Prüfen, ob alle E-Mails weitergeleitet wurden.",
                  "shortLink": "WtgfKH5P"
                },
                "list": null,
                "listAfter": {
                  "id": "5fce1e1ebb7b5d587c848801",
                  "name": "Sprint"
                },
                "listBefore": {
                  "id": "602a503eb52c7978da17bbc5",
                  "name": "Neue Ideen"
                },
                "old": {
                  "idList": "602a503eb52c7978da17bbc5"
                }
              },
              "memberCreator": {
                "id": "5fc5fc01b18f8769073220f0",
                "username": "u",
                "fullName": "U",
                "initials": "u",
                "avatarUrl": "https://trello-members.s3.amazonaws.com/5fc5fc01b18f8769073220f0/1801e1bd09bf6667c25014645216a091",
                "avatarHash": "1801e1bd09bf6667c25014645216a091",
                "activityBlocked": false,
                "idMemberReferrer": "5fbef93856a5065a7a83524b",
                "nonPublic": {},
                "nonPublicAvailable": true
              },
              "limits": null
            }
          ]
        });

        let card: Card = serde_json::from_value(json).unwrap();

        let last_update = last_update_from_card(&card);
        let last_moved = moved_to_list_date(&card).unwrap();

        assert_eq!(
            last_update,
            datetime!(2025-11-21 11:50:59.295 +00:00),
            "last update date mismatch"
        );
        assert_eq!(
            last_moved,
            datetime!(2025-10-27 12:38:36.472 +00:00),
            "last moved date mismatch"
        );
    }
}
