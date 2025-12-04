use crate::{
    config::ActionConfig,
    schema::List,
    slack::SlackMessagePoster,
    trello::{TrelloClient, last_update_from_card, moved_to_list_date},
    util::setup_tracing,
};
use anyhow::Result;
use clap::Parser;
use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
    hash::Hash,
};
use time::OffsetDateTime;
use tracing::{error, info};
use url::Url;

mod config;
mod schema;
mod slack;
mod trello;
mod util;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrelloUser(pub String);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SlackUser(pub String);

impl Display for TrelloUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for SlackUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing();
    dotenvy::dotenv().ok(); // load .env file

    let config = config::AppConfig::parse();

    let trello_to_slack_mapping: HashMap<TrelloUser, SlackUser> = config
        .user_mapping
        .iter()
        .map(|mapping| (mapping.trello_user.clone(), mapping.slack_user.clone()))
        .collect();

    let request_client = reqwest::Client::new();
    let trello_client = TrelloClient::new(request_client.clone(), &config.trello);

    let mut members = HashSet::new();
    for board_id in &config.trello.board_ids {
        let board_members = trello_client.get_members(board_id).await?;
        members.extend(board_members);
    }

    let trello_member_id_to_username: HashMap<String, TrelloUser> = members
        .into_iter()
        .map(|member| (member.id.clone(), TrelloUser(member.username.clone())))
        .collect();

    let mut lists = Vec::new();
    for board in &config.trello.board_ids {
        let board_lists = trello_client.get_lists(board).await?;

        lists.extend(board_lists);
    }

    let slack_poster = SlackMessagePoster::new(request_client.clone(), &config.slack);

    match config.action {
        ActionConfig::PendingReviews => {
            if config.trello.review_lists.is_empty() {
                error!("No review lists configured, cannot proceed with pending reviews action");
                return Ok(());
            }
            pending_reviews(
                &trello_client,
                &slack_poster,
                &trello_to_slack_mapping,
                &trello_member_id_to_username,
                lists
                    .iter()
                    .filter(|list| config.trello.review_lists.contains(&list.name)),
            )
            .await
        }
        ActionConfig::InactiveCards => {
            inactive_cards(
                &trello_client,
                &slack_poster,
                &trello_to_slack_mapping,
                &trello_member_id_to_username,
                lists
                    .iter()
                    .filter(|list| config.trello.inactive_cards_lists.contains(&list.name)),
            )
            .await
        }
    }
}

/// ACTION: Send notifications for pending reviews
async fn pending_reviews(
    trello_client: &TrelloClient,
    slack_poster: &SlackMessagePoster,
    trello_to_slack_mapping: &HashMap<TrelloUser, SlackUser>,
    trello_member_id_to_username: &HashMap<String, TrelloUser>,
    target_lists: impl Iterator<Item = &List>,
) -> Result<()> {
    let pending_reviews =
        get_pending_reviews(trello_client, trello_member_id_to_username, target_lists).await?;

    for (trello_user, pending_reviews) in pending_reviews {
        if pending_reviews.is_empty() {
            continue;
        }
        let Some(slack_user) = trello_to_slack_mapping.get(&trello_user) else {
            error!(
                "No Slack user mapping found for Trello user {trello_user}, skipping notification",
            );
            continue;
        };

        let markdown_text = compose_pending_reviews_message(pending_reviews)?;

        info!(
            "Sending pending reviews notification to Slack user {slack_user} for Trello user {trello_user}"
        );

        slack_poster
            .post_message(slack_user, &markdown_text)
            .await?;
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct PendingReview {
    card_name: String,
    card_url: Url,
    pending_since_days: usize,
}

async fn get_pending_reviews(
    trello_client: &TrelloClient,
    trello_member_id_to_username: &HashMap<String, TrelloUser>,
    target_lists: impl Iterator<Item = &List>,
) -> Result<HashMap<TrelloUser, Vec<PendingReview>>> {
    let mut pending_reviews = HashMap::<TrelloUser, Vec<PendingReview>>::new();

    for list in target_lists {
        info!("Processing list '{}' (ID: {})", list.name, list.id);

        let cards = trello_client.get_cards(&list.id).await?;

        for card in &cards {
            let trello_users = card
                .id_members
                .iter()
                .filter_map(|user_id| {
                    let trello_user = trello_member_id_to_username.get(user_id).cloned();

                    if trello_user.is_none() {
                        error!("Could not find Trello user for member ID {user_id}");
                    }

                    trello_user
                })
                .collect::<Vec<_>>();

            if trello_users.is_empty() {
                info!(
                    "Skipping card '{}' (ID: {}) with no mapped Trello users",
                    card.name, card.id
                );
                continue;
            }

            let last_update = last_update_from_card(card);

            let pending_review = PendingReview {
                card_name: card.name.clone(),
                card_url: card.url.clone(),
                pending_since_days: (OffsetDateTime::now_utc() - last_update).whole_days() as usize,
            };
            for trello_user in trello_users {
                pending_reviews
                    .entry(trello_user)
                    .or_default()
                    .push(pending_review.clone());
            }
        }
    }

    Ok(pending_reviews)
}

fn compose_pending_reviews_message(mut pending_reviews: Vec<PendingReview>) -> Result<String> {
    pending_reviews.sort_by_key(|review| usize::MAX - review.pending_since_days); // descending

    let mut markdown_text = String::new();
    writeln!(
        &mut markdown_text,
        "**🔎 Du hast {} ausstehende{s1} Review{s2}:**",
        pending_reviews.len(),
        s1 = if pending_reviews.len() == 1 { "s" } else { "" },
        s2 = if pending_reviews.len() > 1 { "s" } else { "" },
    )?;
    for PendingReview {
        card_name,
        card_url,
        pending_since_days,
    } in pending_reviews
    {
        write!(&mut markdown_text, "- [{card_name}]({card_url})")?;
        if pending_since_days >= 1 {
            write!(
                &mut markdown_text,
                " - Wartet seit {pending_since_days} Tag{en} {sirens}",
                en = if pending_since_days > 1 { "en" } else { "" },
                sirens = "🚨".repeat(pending_since_days.saturating_sub(1))
            )?;
        }
        writeln!(&mut markdown_text)?;
    }
    writeln!(&mut markdown_text, "\n\n")?;
    writeln!(
        &mut markdown_text,
        "Mach das Team glücklich und bearbeite das zeitnah!"
    )?;

    Ok(markdown_text)
}

const INACTIVE_WEEKS_THRESHOLD: usize = 2;

/// ACTION: Send notifications for inactive cards
async fn inactive_cards(
    trello_client: &TrelloClient,
    slack_poster: &SlackMessagePoster,
    trello_to_slack_mapping: &HashMap<TrelloUser, SlackUser>,
    trello_member_id_to_username: &HashMap<String, TrelloUser>,
    target_lists: impl Iterator<Item = &List>,
) -> Result<()> {
    let inactive_cards =
        get_inactive_cards(trello_client, trello_member_id_to_username, target_lists).await?;

    for (trello_user, inactive_cards) in inactive_cards {
        if inactive_cards.is_empty() {
            continue;
        }
        let Some(slack_user) = trello_to_slack_mapping.get(&trello_user) else {
            error!(
                "No Slack user mapping found for Trello user {trello_user}, skipping notification",
            );
            continue;
        };

        info!(
            "Sending inactive cards notification to Slack user {slack_user} for Trello user {trello_user}"
        );

        let markdown_text = compose_inactive_cards_message(inactive_cards)?;
        slack_poster
            .post_message(slack_user, &markdown_text)
            .await?;
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct InactiveCard {
    card_name: String,
    card_url: Url,
    pending_since_weeks: usize,
}

async fn get_inactive_cards(
    trello_client: &TrelloClient,
    trello_member_id_to_username: &HashMap<String, TrelloUser>,
    target_lists: impl Iterator<Item = &List>,
) -> Result<HashMap<TrelloUser, Vec<InactiveCard>>> {
    let mut inactive_cards = HashMap::<TrelloUser, Vec<InactiveCard>>::new();

    for list in target_lists {
        info!("Processing list '{}' (ID: {})", list.name, list.id);

        let cards = trello_client.get_cards(&list.id).await?;

        for card in &cards {
            let trello_users = card
                .id_members
                .iter()
                .filter_map(|user_id| {
                    let trello_user = trello_member_id_to_username.get(user_id).cloned();

                    if trello_user.is_none() {
                        error!("Could not find Trello user for member ID {user_id}");
                    }

                    trello_user
                })
                .collect::<Vec<_>>();

            if trello_users.is_empty() {
                info!(
                    "Skipping card '{}' (ID: {}) with no mapped Trello users",
                    card.name, card.id
                );
                continue;
            }

            let in_list_since = moved_to_list_date(card)?;

            let inactive_card = InactiveCard {
                card_name: card.name.clone(),
                card_url: card.url.clone(),
                pending_since_weeks: (OffsetDateTime::now_utc() - in_list_since).whole_weeks()
                    as usize,
            };

            if inactive_card.pending_since_weeks < INACTIVE_WEEKS_THRESHOLD {
                continue; // not inactive enough
            }

            for trello_user in trello_users {
                inactive_cards
                    .entry(trello_user)
                    .or_default()
                    .push(inactive_card.clone());
            }
        }
    }

    Ok(inactive_cards)
}

fn compose_inactive_cards_message(mut inactive_cards: Vec<InactiveCard>) -> Result<String> {
    inactive_cards.sort_by_key(|card| usize::MAX - card.pending_since_weeks); // descending

    let mut markdown_text = String::new();
    writeln!(
        &mut markdown_text,
        "**📝 Folgende {number} Karte{n} {is} seit längerer Zeit im Sprint:**",
        number = inactive_cards.len(),
        n = if inactive_cards.len() > 1 { "n" } else { "" },
        is = if inactive_cards.len() > 1 {
            "sind"
        } else {
            "ist"
        },
    )?;
    for InactiveCard {
        card_name,
        card_url,
        pending_since_weeks,
    } in inactive_cards
    {
        writeln!(
            &mut markdown_text,
            "- [{card_name}]({card_url}) - In Liste seit {pending_since_weeks} Wochen {sirens}",
            sirens = "🚨".repeat(pending_since_weeks.saturating_sub(INACTIVE_WEEKS_THRESHOLD))
        )?;
    }
    writeln!(&mut markdown_text, "\n\n")?;
    writeln!(
        &mut markdown_text,
        "Schau mal nach, ob die Karten zu bearbeiten sind!"
    )?;

    Ok(markdown_text)
}
