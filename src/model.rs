use chrono::{DateTime, Duration, Local};

#[derive(Clone, Debug)]
pub struct Conversation {
    pub id: i32,
    pub name: String,
    pub participants: Vec<String>,
    pub last_date: DateTime<Local>,
}

impl Conversation {
    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }

        let values = || {
            std::iter::once(self.name.as_str()).chain(self.participants.iter().map(String::as_str))
        };
        if values().any(|value| value.to_lowercase().contains(&query)) {
            return true;
        }

        let query_digits: String = query.chars().filter(char::is_ascii_digit).collect();
        !query_digits.is_empty()
            && values()
                .map(|value| {
                    value
                        .chars()
                        .filter(char::is_ascii_digit)
                        .collect::<String>()
                })
                .any(|digits| digits.contains(&query_digits))
    }
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

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::Conversation;

    fn conversation(name: &str, participants: &[&str]) -> Conversation {
        Conversation {
            id: 1,
            name: name.to_string(),
            participants: participants.iter().map(|value| value.to_string()).collect(),
            last_date: Local::now(),
        }
    }

    #[test]
    fn search_matches_contact_names_case_insensitively() {
        let conversation = conversation("Alice Smith", &["+18455551212"]);
        assert!(conversation.matches_search("alice"));
        assert!(conversation.matches_search("SMITH"));
        assert!(!conversation.matches_search("Bob"));
    }

    #[test]
    fn search_matches_phone_numbers_ignoring_formatting() {
        let conversation = conversation("Alice Smith", &["+1 (845) 555-1212"]);
        assert!(conversation.matches_search("845-555"));
        assert!(conversation.matches_search("5551212"));
        assert!(!conversation.matches_search("5559999"));
    }

    #[test]
    fn empty_search_matches_every_conversation() {
        assert!(conversation("Alice", &[]).matches_search(""));
    }
}
