use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::model::{ChatMessage, Conversation, ExportRange};

pub const TUI_EXPORT_DIRECTORY: &str = "exports";
pub const GUI_EXPORT_DIRECTORY: &str = "iMessage Exports";

pub fn default_export_path(
    directory: &Path,
    conversation: &Conversation,
    range: &ExportRange,
) -> PathBuf {
    let filename = format!(
        "{}-{}-{}.md",
        safe_filename(&conversation.name),
        range.label(),
        chrono::Local::now().format("%Y-%m-%d")
    );
    directory.join(filename)
}

pub fn write_markdown(
    path: &Path,
    conversation: &Conversation,
    range: &ExportRange,
    messages: &[ChatMessage],
) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Unable to create export directory {}", parent.display()))?;
    }

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
    use std::{fs, path::PathBuf};

    use chrono::{Local, TimeZone};

    use crate::model::ChatMessage;

    use super::{default_export_path, message_header, safe_filename, write_markdown};

    fn conversation() -> crate::model::Conversation {
        crate::model::Conversation {
            id: 1,
            name: "Demo Contact".to_string(),
            participants: vec!["+15555550123".to_string()],
            last_date: Local::now(),
        }
    }

    #[test]
    fn sanitizes_filename() {
        assert_eq!(safe_filename("Sarah / Family Chat"), "Sarah-_-Family-Chat");
    }

    #[test]
    fn default_path_stays_inside_the_requested_directory() {
        let directory = PathBuf::from("exports");
        let path = default_export_path(
            &directory,
            &conversation(),
            &crate::model::ExportRange::LastHour,
        );

        assert_eq!(path.parent(), Some(directory.as_path()));
        assert!(
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("Demo-Contact-last-1-hour-"))
        );
    }

    #[test]
    fn writing_an_export_creates_its_parent_directory() {
        let root = std::env::temp_dir().join(format!(
            "imessage-tui-export-test-{}-{}",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let path = root.join("nested").join("messages.md");

        write_markdown(
            &path,
            &conversation(),
            &crate::model::ExportRange::LastHour,
            &[],
        )
        .unwrap();

        assert!(path.is_file());
        fs::remove_dir_all(root).unwrap();
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
