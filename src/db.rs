use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use imessage_database::{
    message_types::variants::{TapbackAction, Variant},
    tables::{
        handle::Handle,
        messages::Message,
        table::{Cacheable, Table, get_connection},
    },
    util::{
        dates::{get_local_time, get_offset},
        dirs::default_db_path,
    },
};
use rusqlite::{Connection, OpenFlags, params};

use crate::model::{ChatMessage, Conversation};

const MESSAGE_QUERY_HEAD: &str = r#"
SELECT
    m.*,
    c.chat_id,
    (SELECT COUNT(*) FROM message_attachment_join a WHERE m.ROWID = a.message_id) AS num_attachments,
    d.chat_id AS deleted_from,
    (SELECT COUNT(*) FROM message m2 WHERE m2.thread_originator_guid = m.guid) AS num_replies
FROM message AS m
LEFT JOIN chat_message_join AS c ON m.ROWID = c.message_id
LEFT JOIN chat_recoverable_message_join AS d ON m.ROWID = d.message_id
"#;

pub struct Database {
    connection: Connection,
    contacts: ContactsIndex,
    handles: HashMap<i32, String>,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        Self::open(&default_db_path(), true)
    }

    fn open(path: &Path, load_contacts: bool) -> Result<Self> {
        let connection = get_connection(path).map_err(anyhow::Error::msg)?;
        let handles = Handle::cache(&connection).map_err(anyhow::Error::msg)?;
        let contacts = if load_contacts {
            ContactsIndex::load()
        } else {
            ContactsIndex::default()
        };
        Ok(Self {
            connection,
            contacts,
            handles,
        })
    }

    pub fn conversations(&self) -> Result<Vec<Conversation>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT
                c.ROWID,
                c.chat_identifier,
                c.display_name,
                MAX(m.date) AS last_date,
                GROUP_CONCAT(DISTINCT h.id) AS participants
            FROM chat AS c
            JOIN chat_message_join AS cmj ON cmj.chat_id = c.ROWID
            JOIN message AS m ON m.ROWID = cmj.message_id
            LEFT JOIN chat_handle_join AS chj ON chj.chat_id = c.ROWID
            LEFT JOIN handle AS h ON h.ROWID = chj.handle_id
            GROUP BY c.ROWID
            ORDER BY last_date DESC
            "#,
        )?;

        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;

        let mut conversations = Vec::new();
        for row in rows {
            let (id, identifier, display_name, raw_date, participants_raw) = row?;
            let participant_ids: Vec<String> = participants_raw
                .unwrap_or_else(|| identifier.clone())
                .split(',')
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            let participants: Vec<String> = participant_ids
                .iter()
                .map(|participant| self.contacts.resolve(participant))
                .collect();
            let name = display_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| {
                    if participants.is_empty() {
                        self.contacts.resolve(&identifier)
                    } else {
                        participants.join(", ")
                    }
                });
            let last_date = get_local_time(raw_date, get_offset())
                .with_context(|| format!("Invalid timestamp for conversation {id}"))?;
            conversations.push(Conversation {
                id,
                name,
                participants,
                last_date,
            });
        }
        Ok(conversations)
    }

    pub fn message_page(
        &self,
        chat_id: i32,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ChatMessage>> {
        let query = format!(
            "{MESSAGE_QUERY_HEAD} WHERE c.chat_id = ?1 ORDER BY m.date DESC LIMIT ?2 OFFSET ?3"
        );
        let mut statement = self.connection.prepare(&query)?;
        let rows = Message::rows(
            &mut statement,
            params![chat_id, i64::try_from(limit)?, i64::try_from(offset)?],
        )
        .map_err(anyhow::Error::msg)?;
        let mut messages = self.collect_messages(rows)?;
        messages.reverse();
        Ok(messages)
    }

    pub fn messages_since(
        &self,
        chat_id: i32,
        start: Option<DateTime<Local>>,
    ) -> Result<Vec<ChatMessage>> {
        let start_stamp = start.map(messages_timestamp);
        let query = format!(
            "{MESSAGE_QUERY_HEAD} WHERE c.chat_id = ?1 AND (?2 IS NULL OR m.date >= ?2) ORDER BY m.date"
        );
        let mut statement = self.connection.prepare(&query)?;
        let rows = Message::rows(&mut statement, params![chat_id, start_stamp])
            .map_err(anyhow::Error::msg)?;
        self.collect_messages(rows)
    }

    fn collect_messages<'a>(
        &self,
        rows: impl Iterator<Item = Result<Message, imessage_database::error::table::TableError>> + 'a,
    ) -> Result<Vec<ChatMessage>> {
        let mut out = Vec::new();
        for row in rows {
            let mut message = row.map_err(anyhow::Error::msg)?;
            if let Ok(body) = message.parse_body(&self.connection) {
                message.apply_body(body);
            }
            let sender = if message.is_from_me {
                "Me".to_string()
            } else {
                message
                    .handle_id
                    .and_then(|id| self.handles.get(&id))
                    .map(|handle| self.contacts.resolve(handle))
                    .unwrap_or_else(|| "Unknown".to_string())
            };
            let reaction = match message.variant() {
                Variant::Tapback(_, action, tapback) => {
                    let verb = match action {
                        TapbackAction::Added => "added",
                        TapbackAction::Removed => "removed",
                    };
                    Some(format!("{verb} {tapback}"))
                }
                _ => None,
            };
            let date = get_local_time(message.date, get_offset())
                .with_context(|| format!("Invalid message timestamp: {}", message.date))?;
            out.push(ChatMessage {
                date,
                sender,
                text: message.text,
                reaction,
                attachment_count: usize::try_from(message.num_attachments).unwrap_or(0),
            });
        }
        Ok(out)
    }
}

fn messages_timestamp(date: DateTime<Local>) -> i64 {
    (date.timestamp() - get_offset())
        .saturating_mul(1_000_000_000)
        .saturating_add(i64::from(date.timestamp_subsec_nanos()))
}

#[derive(Default)]
struct ContactsIndex {
    names: HashMap<String, String>,
}

impl ContactsIndex {
    fn load() -> Self {
        let mut index = Self::default();
        let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
            return index;
        };
        let sources = home.join("Library/Application Support/AddressBook/Sources");
        let Ok(entries) = fs::read_dir(sources) else {
            return index;
        };
        for entry in entries.flatten() {
            let path = entry.path().join("AddressBook-v22.abcddb");
            if path.is_file() {
                let _ = index.load_database(path);
            }
        }
        index
    }

    fn load_database(&mut self, path: PathBuf) -> Result<()> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let mut statement = connection.prepare(
            r#"
            SELECT r.ZFIRSTNAME, r.ZLASTNAME, p.ZFULLNUMBER, e.ZADDRESSNORMALIZED
            FROM ZABCDRECORD AS r
            LEFT JOIN ZABCDPHONENUMBER AS p ON r.Z_PK = p.ZOWNER
            LEFT JOIN ZABCDEMAILADDRESS AS e ON r.Z_PK = e.ZOWNER
            "#,
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows.flatten() {
            let (first, last, phone, email) = row;
            let name = [first, last]
                .into_iter()
                .flatten()
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if name.is_empty() {
                continue;
            }
            if let Some(phone) = phone {
                for key in identifier_keys(&phone) {
                    self.names.entry(key).or_insert_with(|| name.clone());
                }
            }
            if let Some(email) = email {
                for raw in email.split_whitespace() {
                    for key in identifier_keys(raw) {
                        self.names.entry(key).or_insert_with(|| name.clone());
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve(&self, identifier: &str) -> String {
        for key in identifier_keys(identifier) {
            if let Some(name) = self.names.get(&key) {
                return name.clone();
            }
        }
        identifier.to_string()
    }
}

fn identifier_keys(identifier: &str) -> Vec<String> {
    let trimmed = identifier.trim().trim_matches(['<', '>']).to_lowercase();
    if trimmed.contains('@') {
        return vec![trimmed];
    }
    let digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return vec![trimmed];
    }
    let mut keys = vec![digits.clone()];
    if digits.len() > 10 {
        keys.push(digits[digits.len() - 10..].to_string());
    }
    if digits.len() > 7 {
        keys.push(digits[digits.len() - 7..].to_string());
    }
    keys
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use rusqlite::Connection;

    use super::{Database, identifier_keys};

    #[test]
    fn normalizes_email() {
        assert_eq!(
            identifier_keys("<Sarah@Example.COM>"),
            ["sarah@example.com"]
        );
    }

    #[test]
    fn creates_phone_suffixes() {
        assert_eq!(
            identifier_keys("+1 (845) 555-1212"),
            ["18455551212", "8455551212", "5551212"]
        );
    }

    #[test]
    fn reads_conversation_and_pages_messages_from_fixture() {
        let registry_root = PathBuf::from(env::var("HOME").unwrap()).join(".cargo/registry/src");
        let source = fs::read_dir(registry_root).ok().and_then(|entries| {
            entries.flatten().find_map(|entry| {
                let candidate = entry
                    .path()
                    .join("imessage-database-4.2.0/test_data/db/test.db");
                candidate.exists().then_some(candidate)
            })
        });
        let Some(source) = source else {
            // Source distributions may omit dependency test fixtures.
            return;
        };
        let target = env::temp_dir().join(format!("imessage-tui-test-{}.db", std::process::id()));
        fs::copy(&source, &target).unwrap();
        {
            let connection = Connection::open(&target).unwrap();
            connection
                .execute(
                    "INSERT INTO handle (ROWID, id, service) VALUES (1, '+18455551212', 'iMessage')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO chat (ROWID, guid, chat_identifier, service_name) VALUES (1, 'fixture-chat', '+18455551212', 'iMessage')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 1)",
                    [],
                )
                .unwrap();
            for message_id in [123445_i64, 452567, 548216] {
                connection
                    .execute(
                        "INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, ?1)",
                        [message_id],
                    )
                    .unwrap();
            }
        }

        let database = Database::open(&target, false).unwrap();
        let conversations = database.conversations().unwrap();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].name, "+18455551212");
        let newest = database.message_page(1, 2, 0).unwrap();
        let older = database.message_page(1, 2, 2).unwrap();
        assert_eq!(newest.len(), 2);
        assert_eq!(older.len(), 1);
        assert!(newest[0].date <= newest[1].date);

        fs::remove_file(target).unwrap();
    }
}
