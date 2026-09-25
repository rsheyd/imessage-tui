use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::model::{ChatMessage, Conversation, ExportRange};

pub const TUI_EXPORT_DIRECTORY: &str = "exports";
pub const GUI_EXPORT_DIRECTORY: &str = "iMessage Exports";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExportSummary {
    pub copied_images: usize,
    pub unavailable_images: usize,
}

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
) -> Result<ExportSummary> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Unable to create export directory {}", parent.display()))?;
    }

    let mut output = String::new();
    let image_directory_name = format!(
        "{}-images",
        safe_filename(
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("messages")
        )
    );
    let image_directory = path.with_file_name(&image_directory_name);
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
    let mut summary = ExportSummary::default();
    let mut body = String::new();

    if messages.is_empty() {
        body.push_str("_No messages in this range._\n");
    } else {
        for message in messages {
            let mut image_markdown = Vec::new();
            for image in &message.images {
                let filename = format!("image-{}.{}", image.id, image.extension);
                let destination = image_directory.join(&filename);
                if !image_directory.exists() {
                    fs::create_dir_all(&image_directory).with_context(|| {
                        format!(
                            "Unable to create image directory {}",
                            image_directory.display()
                        )
                    })?;
                }
                if fs::copy(&image.source, &destination).is_ok() {
                    summary.copied_images += 1;
                    image_markdown.push(format!(
                        "![Attached image]({image_directory_name}/{filename})"
                    ));
                } else {
                    summary.unavailable_images += 1;
                }
            }
            body.push_str(&message_header(message));
            body.push_str(&message.display_body_with_images(&image_markdown));
            body.push_str("\n\n");
        }
    }

    output.push_str(&format!(
        "- Images: {} copied, {} unavailable\n\n",
        summary.copied_images, summary.unavailable_images
    ));
    output.push_str(&body);
    fs::write(path, output)
        .with_context(|| format!("Unable to write export to {}", path.display()))?;
    Ok(summary)
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

    use crate::model::{ChatMessage, ImageAttachment};

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
            images: vec![],
        };

        assert_eq!(
            message_header(&message),
            "**2026-07-16 12:15:27 — Demo Contact**\n\n"
        );
    }

    #[test]
    fn copies_selected_message_images_and_keeps_missing_attachment_visible() {
        let root = std::env::temp_dir().join(format!(
            "imessage-tui-images-test-{}-{}",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        fs::write(&source, b"test image").unwrap();
        let path = root.join("out").join("conversation.md");
        let message = ChatMessage {
            date: Local::now(),
            sender: "Me".to_string(),
            text: Some("Here it is".to_string()),
            reaction: None,
            attachment_count: 3,
            images: vec![
                ImageAttachment {
                    id: 42,
                    source: source.clone(),
                    extension: "png".to_string(),
                },
                ImageAttachment {
                    id: 43,
                    source: root.join("missing.png"),
                    extension: "png".to_string(),
                },
            ],
        };

        let summary = write_markdown(
            &path,
            &conversation(),
            &crate::model::ExportRange::LastHour,
            &[message],
        )
        .unwrap();

        let markdown = fs::read_to_string(&path).unwrap();
        assert_eq!(summary.copied_images, 1);
        assert_eq!(summary.unavailable_images, 1);
        assert!(markdown.contains("- Images: 1 copied, 1 unavailable"));
        assert!(markdown.contains("![Attached image](conversation-images/image-42.png)"));
        assert!(markdown.contains("[2 attachments]"));
        assert!(!markdown.contains("image-43.png"));
        assert_eq!(
            fs::read(
                path.parent()
                    .unwrap()
                    .join("conversation-images/image-42.png")
            )
            .unwrap(),
            b"test image"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
