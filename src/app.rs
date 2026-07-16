use std::{env, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::{
    db::Database,
    export::{safe_filename, write_markdown},
    model::{ChatMessage, Conversation, ExportRange},
};

const PAGE_SIZE: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Conversations,
    Messages,
}

pub enum Modal {
    None,
    ExportMenu { selected: usize },
    NumberInput { unit: InputUnit, value: String },
    PathInput { range: ExportRange, value: String },
    Notice(String),
}

#[derive(Clone, Copy)]
pub enum InputUnit {
    Hours,
    Days,
}

pub struct App {
    pub db: Database,
    pub conversations: Vec<Conversation>,
    pub visible_conversation_indices: Vec<usize>,
    pub conversation_state: ListState,
    pub conversation_search: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub message_state: ListState,
    pub screen: Screen,
    pub modal: Modal,
    pub should_quit: bool,
    pub loaded_all: bool,
    pub status: String,
}

impl App {
    pub fn new() -> Result<Self> {
        let db = Database::open_default()?;
        let conversations = db.conversations()?;
        let visible_conversation_indices = (0..conversations.len()).collect();
        let mut conversation_state = ListState::default();
        if !conversations.is_empty() {
            conversation_state.select(Some(0));
        }
        Ok(Self {
            db,
            conversations,
            visible_conversation_indices,
            conversation_state,
            conversation_search: None,
            messages: Vec::new(),
            message_state: ListState::default(),
            screen: Screen::Conversations,
            modal: Modal::None,
            should_quit: false,
            loaded_all: false,
            status: "Enter: open  •  /: search  •  q: quit".to_string(),
        })
    }

    pub fn current_conversation(&self) -> Option<&Conversation> {
        self.conversation_state
            .selected()
            .and_then(|index| self.visible_conversation_indices.get(index))
            .and_then(|index| self.conversations.get(*index))
    }

    pub fn visible_conversations(&self) -> impl Iterator<Item = &Conversation> {
        self.visible_conversation_indices
            .iter()
            .filter_map(|index| self.conversations.get(*index))
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match std::mem::replace(&mut self.modal, Modal::None) {
            Modal::None => self.handle_screen_key(key),
            Modal::ExportMenu { selected } => self.handle_export_menu(key, selected),
            Modal::NumberInput { unit, value } => self.handle_number_input(key, unit, value),
            Modal::PathInput { range, value } => self.handle_path_input(key, range, value),
            Modal::Notice(_) => {
                self.modal = Modal::None;
                Ok(())
            }
        }
    }

    fn handle_screen_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.screen {
            Screen::Conversations if self.conversation_search.is_some() => {
                self.handle_search_key(key)?
            }
            Screen::Conversations => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('/') => {
                    self.conversation_search = Some(String::new());
                    self.update_conversation_filter();
                }
                KeyCode::Up | KeyCode::Char('k') => move_selection(
                    &mut self.conversation_state,
                    self.visible_conversation_indices.len(),
                    -1,
                ),
                KeyCode::Down | KeyCode::Char('j') => move_selection(
                    &mut self.conversation_state,
                    self.visible_conversation_indices.len(),
                    1,
                ),
                KeyCode::PageUp => move_selection(
                    &mut self.conversation_state,
                    self.visible_conversation_indices.len(),
                    -10,
                ),
                KeyCode::PageDown => move_selection(
                    &mut self.conversation_state,
                    self.visible_conversation_indices.len(),
                    10,
                ),
                KeyCode::Enter => self.open_conversation()?,
                _ => {}
            },
            Screen::Messages => match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => self.close_conversation(),
                KeyCode::Up | KeyCode::Char('k') => self.older(1)?,
                KeyCode::Down | KeyCode::Char('j') => self.newer(1),
                KeyCode::PageUp => self.older(10)?,
                KeyCode::PageDown => self.newer(10),
                KeyCode::Home => {
                    while !self.loaded_all {
                        self.load_older()?;
                    }
                    self.message_state
                        .select((!self.messages.is_empty()).then_some(0));
                }
                KeyCode::End => self
                    .message_state
                    .select(self.messages.len().checked_sub(1)),
                KeyCode::Char('e') => self.modal = Modal::ExportMenu { selected: 0 },
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.conversation_search = None;
                self.update_conversation_filter();
            }
            KeyCode::Enter => self.open_conversation()?,
            KeyCode::Up => move_selection(
                &mut self.conversation_state,
                self.visible_conversation_indices.len(),
                -1,
            ),
            KeyCode::Down => move_selection(
                &mut self.conversation_state,
                self.visible_conversation_indices.len(),
                1,
            ),
            KeyCode::PageUp => move_selection(
                &mut self.conversation_state,
                self.visible_conversation_indices.len(),
                -10,
            ),
            KeyCode::PageDown => move_selection(
                &mut self.conversation_state,
                self.visible_conversation_indices.len(),
                10,
            ),
            KeyCode::Backspace => {
                if let Some(query) = &mut self.conversation_search {
                    query.pop();
                }
                self.update_conversation_filter();
            }
            KeyCode::Char(ch) => {
                if let Some(query) = &mut self.conversation_search {
                    query.push(ch);
                }
                self.update_conversation_filter();
            }
            _ => {}
        }
        Ok(())
    }

    fn update_conversation_filter(&mut self) {
        let query = self.conversation_search.as_deref().unwrap_or_default();
        self.visible_conversation_indices = self
            .conversations
            .iter()
            .enumerate()
            .filter_map(|(index, conversation)| {
                conversation_matches(conversation, query).then_some(index)
            })
            .collect();
        self.conversation_state
            .select((!self.visible_conversation_indices.is_empty()).then_some(0));
        self.status = match &self.conversation_search {
            Some(query) => format!(
                "Search: {query}_  •  {} match{}  •  Enter: open  •  Esc: clear",
                self.visible_conversation_indices.len(),
                if self.visible_conversation_indices.len() == 1 {
                    ""
                } else {
                    "es"
                }
            ),
            None => "Enter: open  •  /: search  •  q: quit".to_string(),
        };
    }

    fn open_conversation(&mut self) -> Result<()> {
        let Some(conversation) = self.current_conversation() else {
            return Ok(());
        };
        self.messages = self.db.message_page(conversation.id, PAGE_SIZE, 0)?;
        self.loaded_all = self.messages.len() < PAGE_SIZE;
        self.message_state
            .select(self.messages.len().checked_sub(1));
        self.screen = Screen::Messages;
        self.status = "↑/↓: messages  •  PgUp/PgDn  •  e: export  •  q/Esc: back".to_string();
        Ok(())
    }

    fn close_conversation(&mut self) {
        self.screen = Screen::Conversations;
        self.messages.clear();
        self.message_state.select(None);
        self.status = if let Some(query) = &self.conversation_search {
            let count = self.visible_conversation_indices.len();
            format!(
                "Search: {query}_  •  {count} match{}  •  Enter: open  •  Esc: clear",
                if count == 1 { "" } else { "es" }
            )
        } else {
            "Enter: open  •  /: search  •  q: quit".to_string()
        };
    }

    fn older(&mut self, amount: usize) -> Result<()> {
        if self.messages.is_empty() {
            return Ok(());
        }
        let current = self.message_state.selected().unwrap_or(0);
        if current >= amount {
            self.message_state.select(Some(current - amount));
            return Ok(());
        }
        if !self.loaded_all {
            let previous_len = self.messages.len();
            self.load_older()?;
            let added = self.messages.len() - previous_len;
            self.message_state
                .select(Some(added.saturating_add(current).saturating_sub(amount)));
        } else {
            self.message_state.select(Some(0));
        }
        Ok(())
    }

    fn newer(&mut self, amount: usize) {
        if let Some(current) = self.message_state.selected() {
            let target = current
                .saturating_add(amount)
                .min(self.messages.len().saturating_sub(1));
            self.message_state.select(Some(target));
        }
    }

    fn load_older(&mut self) -> Result<()> {
        let Some(conversation) = self.current_conversation() else {
            return Ok(());
        };
        let mut older = self
            .db
            .message_page(conversation.id, PAGE_SIZE, self.messages.len())?;
        self.loaded_all = older.len() < PAGE_SIZE;
        older.append(&mut self.messages);
        self.messages = older;
        Ok(())
    }

    fn handle_export_menu(&mut self, key: KeyEvent, mut selected: usize) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {}
            KeyCode::Up | KeyCode::Char('k') => {
                selected = selected.saturating_sub(1);
                self.modal = Modal::ExportMenu { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(4);
                self.modal = Modal::ExportMenu { selected };
            }
            KeyCode::Enter => match selected {
                0 => self.begin_path_prompt(ExportRange::LastHour)?,
                1 => self.begin_path_prompt(ExportRange::Last24Hours)?,
                2 => {
                    self.modal = Modal::NumberInput {
                        unit: InputUnit::Hours,
                        value: String::new(),
                    }
                }
                3 => {
                    self.modal = Modal::NumberInput {
                        unit: InputUnit::Days,
                        value: String::new(),
                    }
                }
                _ => self.begin_path_prompt(ExportRange::Everything)?,
            },
            _ => self.modal = Modal::ExportMenu { selected },
        }
        Ok(())
    }

    fn handle_number_input(
        &mut self,
        key: KeyEvent,
        unit: InputUnit,
        mut value: String,
    ) -> Result<()> {
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                value.pop();
                self.modal = Modal::NumberInput { unit, value };
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                value.push(ch);
                self.modal = Modal::NumberInput { unit, value };
            }
            KeyCode::Enter => {
                let number: u64 = value.parse().context("Enter a positive whole number")?;
                if number == 0 {
                    bail!("Enter a number greater than zero");
                }
                let range = match unit {
                    InputUnit::Hours => ExportRange::Hours(number),
                    InputUnit::Days => ExportRange::Days(number),
                };
                self.begin_path_prompt(range)?;
            }
            _ => self.modal = Modal::NumberInput { unit, value },
        }
        Ok(())
    }

    fn begin_path_prompt(&mut self, range: ExportRange) -> Result<()> {
        let conversation = self
            .current_conversation()
            .context("No conversation selected")?;
        let filename = format!(
            "{}-{}-{}.md",
            safe_filename(&conversation.name),
            range.label(),
            Local::now().format("%Y-%m-%d")
        );
        self.modal = Modal::PathInput {
            range,
            value: filename,
        };
        Ok(())
    }

    fn handle_path_input(
        &mut self,
        key: KeyEvent,
        range: ExportRange,
        mut value: String,
    ) -> Result<()> {
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Backspace => {
                value.pop();
                self.modal = Modal::PathInput { range, value };
            }
            KeyCode::Char(ch) => {
                value.push(ch);
                self.modal = Modal::PathInput { range, value };
            }
            KeyCode::Enter => {
                if value.trim().is_empty() {
                    bail!("Export path cannot be empty");
                }
                self.export(range, value)?;
            }
            _ => self.modal = Modal::PathInput { range, value },
        }
        Ok(())
    }

    fn export(&mut self, range: ExportRange, value: String) -> Result<()> {
        let conversation = self
            .current_conversation()
            .context("No conversation selected")?
            .clone();
        let path = PathBuf::from(value.trim());
        let path = if path.is_absolute() {
            path
        } else {
            env::current_dir()?.join(path)
        };
        let start = range.start(Local::now());
        let messages = self.db.messages_since(conversation.id, start)?;
        write_markdown(&path, &conversation, &range, &messages)?;
        self.modal = Modal::Notice(format!(
            "Exported {} messages to\n{}\n\nPress any key to continue.",
            messages.len(),
            path.display()
        ));
        Ok(())
    }
}

fn conversation_matches(conversation: &Conversation, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    let text_match = std::iter::once(conversation.name.as_str())
        .chain(conversation.participants.iter().map(String::as_str))
        .any(|value| value.to_lowercase().contains(&query));
    if text_match {
        return true;
    }

    let query_digits: String = query.chars().filter(char::is_ascii_digit).collect();
    !query_digits.is_empty()
        && std::iter::once(conversation.name.as_str())
            .chain(conversation.participants.iter().map(String::as_str))
            .map(|value| {
                value
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
            })
            .any(|digits| digits.contains(&query_digits))
}

fn move_selection(state: &mut ListState, len: usize, amount: isize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let current = state.selected().unwrap_or(0);
    let next = current.saturating_add_signed(amount).min(len - 1);
    state.select(Some(next));
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;

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
        assert!(conversation_matches(&conversation, "alice"));
        assert!(conversation_matches(&conversation, "SMITH"));
        assert!(!conversation_matches(&conversation, "Bob"));
    }

    #[test]
    fn search_matches_phone_numbers_ignoring_formatting() {
        let conversation = conversation("Alice Smith", &["+1 (845) 555-1212"]);
        assert!(conversation_matches(&conversation, "845-555"));
        assert!(conversation_matches(&conversation, "5551212"));
        assert!(!conversation_matches(&conversation, "5559999"));
    }

    #[test]
    fn empty_search_matches_every_conversation() {
        assert!(conversation_matches(&conversation("Alice", &[]), ""));
    }
}
