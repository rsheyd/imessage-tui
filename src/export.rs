use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::model::{ChatMessage, Conversation, ExportRange};

pub fn write_markdown(
    path: &Path,
    conversation: &Conversation,
    range: &ExportRange,
    messages: &[ChatMessage],
) -> Result<()> {
    let mut output = String::new();
    output.push_str(&format!("# Messages with {}\n\n", conversation.name));
    output.push_str(&format!("- Range: {}\n", range.heading()));
    output.push_str(&format!(
        "- Exported: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z")
    ));
    if !conversation.participants.is_empty() {
        output.push_str(&format!(
            "- Participants: {}\n",
            conversation.participants.join(", ")
        ));
    }
    output.push('\n');

    if messages.is_empty() {
        output.push_str("_No messages in this range._\n");
    } else {
        for message in messages {
            output.push_str(&message_header(message));
            output.push_str(&message.display_body());
            output.push_str("\n\n");
        }
    }

    fs::write(path, output).with_context(|| format!("Unable to write export to {}", path.display()))
}

fn message_header(message: &ChatMessage) -> String {
    format!(
        "**{} — {}**\n\n",
        message.date.format("%Y-%m-%d %H:%M:%S"),
        message.sender
    )
}

pub fn safe_filename(name: &str) -> String {
    let mut result: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else if ch.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();
    while result.contains("--") {
        result = result.replace("--", "-");
    }
    let result = result.trim_matches(['-', '_']);
    if result.is_empty() {
        "messages".to_string()
    } else {
        result.to_string()
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use crate::model::ChatMessage;

    use super::{message_header, safe_filename};

    #[test]
    fn sanitizes_filename() {
        assert_eq!(safe_filename("Sarah / Family Chat"), "Sarah-_-Family-Chat");
    }

    #[test]
    fn bolds_message_header_without_creating_a_heading() {
        let message = ChatMessage {
            date: Local
                .with_ymd_and_hms(2026, 7, 16, 12, 15, 27)
                .single()
                .unwrap(),
            sender: "Demo Contact".to_string(),
            text: None,
            reaction: None,
            attachment_count: 0,
        };

        assert_eq!(
            message_header(&message),
            "**2026-07-16 12:15:27 — Demo Contact**\n\n"
        );
    }
}
