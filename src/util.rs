use crate::{config::AppConfig, schema::Action};
use anyhow::{Context, Result};
use log::info;
use std::io::Write;

pub fn setup_tracing() {
    let env = env_logger::Env::default().filter_or("LOG_LEVEL", "info");

    env_logger::init_from_env(env);
}

pub fn is_sorted_descending(actions: &[Action]) -> bool {
    actions.windows(2).all(|w| w[0].date >= w[1].date)
}

pub fn debug_write_to_file<T: serde::Serialize>(
    data: &T,
    file_path: &str,
    title: &str,
) -> Result<()> {
    if cfg!(not(debug_assertions)) {
        return Ok(());
    }

    let mut file = std::fs::File::create(file_path).context("Failed to create file")?;
    let json = serde_json::to_string_pretty(data).context("Failed to serialize data to JSON")?;
    file.write_all(json.as_bytes())
        .context("Failed to write data to file")?;

    info!("{title} have been written to {file_path}");

    Ok(())
}

pub fn print_summary(config: &AppConfig) {
    use tabled::{builder::Builder, settings::Style};

    let mut builder = Builder::with_capacity(5, 2);
    builder.push_record([
        "Users",
        &config
            .user_mapping
            .iter()
            .map(|m| m.trello_user.0.clone())
            .collect::<Vec<_>>()
            .join("\n"),
    ]);
    builder.push_record(["Trello Boards", &config.trello.board_ids.join("\n")]);
    builder.push_record(["Review Lists", &config.trello.review_lists.join("\n")]);
    builder.push_record([
        "Inactive Cards Lists",
        &config.trello.inactive_cards_lists.join("\n"),
    ]);
    let mut table = builder.build();
    table.with(Style::modern());

    info!(
        "Configuration Summary:\n\
        {table}",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ActionData, ActionType, Board, CardAction, MemberCreator};
    use time::{OffsetDateTime, macros::datetime};

    #[test]
    fn it_checks_action_sorting() {
        fn make_action_from_date(date: OffsetDateTime) -> Action {
            Action {
                id: "test_action".to_string(),
                id_member_creator: "test_member".to_string(),
                date,
                r#type: ActionType::Other("test".to_string()),
                app_creator: None,
                data: ActionData {
                    board: Board {
                        id: "board1".to_string(),
                        name: "Board 1".to_string(),
                        short_link: "SL".to_string(),
                    },
                    card: CardAction {
                        id: "card1".to_string(),
                        name: "Card 1".to_string(),
                        id_list: Some("test_list".to_string()),
                        id_short: 1,
                        short_link: "SL".to_string(),
                    },
                    list: None,
                    list_after: None,
                    list_before: None,
                    old: None,
                },
                member_creator: MemberCreator {
                    id: "member1".to_string(),
                    username: "test_user".to_string(),
                    full_name: "Test User".to_string(),
                    initials: "TU".to_string(),
                    avatar_url: None,
                    avatar_hash: None,
                    activity_blocked: false,
                    id_member_referrer: None,
                    non_public: None,
                    non_public_available: false,
                },
                limits: None,
            }
        }

        assert!(is_sorted_descending(&[
            make_action_from_date(datetime!(2024-06-01 12:00:00 +00:00)),
            make_action_from_date(datetime!(2024-05-01 12:00:00 +00:00)),
            make_action_from_date(datetime!(2024-04-01 12:00:00 +00:00)),
        ]));

        assert!(!is_sorted_descending(&[
            make_action_from_date(datetime!(2024-06-01 12:00:00 +00:00)),
            make_action_from_date(datetime!(2024-04-01 12:00:00 +00:00)),
            make_action_from_date(datetime!(2024-05-01 12:00:00 +00:00)),
        ]));
    }
}
