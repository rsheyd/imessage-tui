use chrono::{DateTime, Duration, Local};

#[derive(Clone, Debug)]
pub struct Conversation {
    pub id: i32,
    pub name: String,
    pub participants: Vec<String>,
    pub last_date: DateTime<Local>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub date: DateTime<Local>,
    pub sender: String,
    pub text: Option<String>,
    pub reaction: Option<String>,
    pub attachment_count: usize,
}

impl ChatMessage {
    pub fn display_body(&self) -> String {
        let mut parts = Vec::new();
        if let Some(text) = self.text.as_deref().filter(|text| !text.trim().is_empty()) {
            parts.push(text.to_string());
        }
        if let Some(reaction) = &self.reaction {
            parts.push(format!("Reaction: {reaction}"));
        }
        if self.attachment_count == 1 {
            parts.push("[Attachment]".to_string());
        } else if self.attachment_count > 1 {
            parts.push(format!("[{} attachments]", self.attachment_count));
        }
        if parts.is_empty() {
            parts.push("[Message without exportable text]".to_string());
        }
        parts.join("\n")
    }
}

#[derive(Clone, Debug)]
pub enum ExportRange {
    LastHour,
    Last24Hours,
    Hours(u64),
    Days(u64),
    Everything,
}

impl ExportRange {
    pub fn label(&self) -> String {
        match self {
            Self::LastHour => "last-1-hour".to_string(),
            Self::Last24Hours => "last-24-hours".to_string(),
            Self::Hours(hours) => format!("last-{hours}-hours"),
            Self::Days(days) => format!("last-{days}-days"),
            Self::Everything => "all".to_string(),
        }
    }

    pub fn heading(&self) -> String {
        match self {
            Self::LastHour => "Last hour".to_string(),
            Self::Last24Hours => "Last 24 hours".to_string(),
            Self::Hours(hours) => format!("Last {hours} hours"),
            Self::Days(days) => format!("Last {days} days"),
            Self::Everything => "All messages".to_string(),
        }
    }

    pub fn start(&self, now: DateTime<Local>) -> Option<DateTime<Local>> {
        match self {
            Self::LastHour => Some(now - Duration::hours(1)),
            Self::Last24Hours => Some(now - Duration::hours(24)),
            Self::Hours(hours) => i64::try_from(*hours)
                .ok()
                .and_then(|value| now.checked_sub_signed(Duration::hours(value))),
            Self::Days(days) => i64::try_from(*days)
                .ok()
                .and_then(|value| now.checked_sub_signed(Duration::days(value))),
            Self::Everything => None,
        }
    }
}
