# Trello to Slack Notification Service

This service monitors Trello boards for pending reviews and inactive cards, sending notifications to users via Slack.
It helps teams stay on top of their tasks and ensures timely reviews and actions on cards.

## Building and Running

To build and run the service, ensure you have Rust installed.
Then, clone the repository and run:

```bash
cargo build --release
cargo run --release
```

### Pending Reviews Notification

The service checks for Trello cards that are pending review and sends a notification to the respective Slack user.

```bash
cargo run --release -- pending-reviews
```

### Inactive Cards Notification

The service identifies Trello cards that have been inactive for a specified duration and notifies the assigned Slack user.

```bash
cargo run --release -- inactive-cards
```

## Configuration

The service can be configured via environment variables:

- `SLACK_BOT_TOKEN`: Your Slack bot token.
- `TRELLO_KEY`: Your Trello API key.
- `TRELLO_TOKEN`: Your Trello API token.
- `USER_MAPPING`: A list of Trello to Slack user mappings in the format `trello_user1=slack_user1,trello_user2=slack_user2`.
- `TRELLO_BOARD_IDS`: Comma-separated list of Trello board IDs to monitor.
- `TRELLO_REVIEW_LISTS`: Comma-separated list of Trello list names that contain review cards.
- `TRELLO_INACTIVE_CARDS_LISTS`: Comma-separated list of Trello list names to check for inactive cards.
- `LOG_LEVEL`: Set the logging level (e.g., `info`, `debug`).
