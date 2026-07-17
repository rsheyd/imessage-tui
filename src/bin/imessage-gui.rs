use std::{env, path::PathBuf};

use chrono::Local;
use eframe::egui::{self, Color32, RichText};
use imessage_tui::{
    db::Database,
    export::{safe_filename, write_markdown},
    model::{ChatMessage, Conversation, ExportRange},
};

const PAGE_SIZE: usize = 20;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "iMessage Browser",
        options,
        Box::new(|_creation_context| Ok(Box::new(GuiApp::new()))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportChoice {
    LastHour,
    Last24Hours,
    Hours,
    Days,
    Everything,
}

impl ExportChoice {
    const ALL: [Self; 5] = [
        Self::LastHour,
        Self::Last24Hours,
        Self::Hours,
        Self::Days,
        Self::Everything,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::LastHour => "Last hour",
            Self::Last24Hours => "Last 24 hours",
            Self::Hours => "Choose number of hours",
            Self::Days => "Choose number of days",
            Self::Everything => "Everything",
        }
    }

    fn range(self, amount: &str) -> Result<ExportRange, String> {
        match self {
            Self::LastHour => Ok(ExportRange::LastHour),
            Self::Last24Hours => Ok(ExportRange::Last24Hours),
            Self::Everything => Ok(ExportRange::Everything),
            Self::Hours | Self::Days => {
                let amount = amount
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| "Enter a positive whole number.".to_string())?;
                if amount == 0 {
                    return Err("Enter a number greater than zero.".to_string());
                }
                if self == Self::Hours {
                    Ok(ExportRange::Hours(amount))
                } else {
                    Ok(ExportRange::Days(amount))
                }
            }
        }
    }
}

struct GuiApp {
    database: Option<Database>,
    conversations: Vec<Conversation>,
    selected: Option<usize>,
    messages: Vec<ChatMessage>,
    loaded_all: bool,
    search: String,
    status: String,
    startup_error: Option<String>,
    export_open: bool,
    export_choice: ExportChoice,
    export_amount: String,
    export_path: String,
}

impl GuiApp {
    fn new() -> Self {
        let mut app = Self {
            database: None,
            conversations: Vec::new(),
            selected: None,
            messages: Vec::new(),
            loaded_all: false,
            search: String::new(),
            status: String::new(),
            startup_error: None,
            export_open: false,
            export_choice: ExportChoice::Last24Hours,
            export_amount: "24".to_string(),
            export_path: String::new(),
        };
        app.connect();
        app
    }

    fn connect(&mut self) {
        self.startup_error = None;
        match Database::open_default().and_then(|database| {
            let conversations = database.conversations()?;
            Ok((database, conversations))
        }) {
            Ok((database, conversations)) => {
                self.database = Some(database);
                self.status = format!("{} conversations", conversations.len());
                self.conversations = conversations;
            }
            Err(error) => {
                self.database = None;
                self.startup_error = Some(format!("{error:#}").replace('→', ">"));
            }
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        self.conversations
            .iter()
            .enumerate()
            .filter_map(|(index, conversation)| {
                conversation.matches_search(&self.search).then_some(index)
            })
            .collect()
    }

    fn select_conversation(&mut self, index: usize) {
        let Some(database) = &self.database else {
            return;
        };
        let Some(conversation) = self.conversations.get(index) else {
            return;
        };
        match database.message_page(conversation.id, PAGE_SIZE, 0) {
            Ok(messages) => {
                self.loaded_all = messages.len() < PAGE_SIZE;
                self.messages = messages;
                self.selected = Some(index);
                self.export_path = default_export_path(conversation, &self.export_choice_range());
                self.status = format!("Loaded {} recent messages", self.messages.len());
            }
            Err(error) => self.status = format!("Unable to load messages: {error:#}"),
        }
    }

    fn load_older(&mut self) {
        let (Some(database), Some(index)) = (&self.database, self.selected) else {
            return;
        };
        let Some(conversation) = self.conversations.get(index) else {
            return;
        };
        match database.message_page(conversation.id, PAGE_SIZE, self.messages.len()) {
            Ok(mut older) => {
                self.loaded_all = older.len() < PAGE_SIZE;
                let added = older.len();
                older.append(&mut self.messages);
                self.messages = older;
                self.status = if added == 0 {
                    "All messages are loaded".to_string()
                } else {
                    format!("Loaded {added} older messages")
                };
            }
            Err(error) => self.status = format!("Unable to load older messages: {error:#}"),
        }
    }

    fn export_choice_range(&self) -> ExportRange {
        self.export_choice
            .range(&self.export_amount)
            .unwrap_or(ExportRange::Last24Hours)
    }

    fn refresh_export_path(&mut self) {
        if let Some(conversation) = self
            .selected
            .and_then(|index| self.conversations.get(index))
        {
            self.export_path = default_export_path(conversation, &self.export_choice_range());
        }
    }

    fn export(&mut self) -> bool {
        let range = match self.export_choice.range(&self.export_amount) {
            Ok(range) => range,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        let (Some(database), Some(index)) = (&self.database, self.selected) else {
            self.status = "Select a conversation first.".to_string();
            return false;
        };
        let Some(conversation) = self.conversations.get(index) else {
            return false;
        };
        let path = PathBuf::from(self.export_path.trim());
        if path.as_os_str().is_empty() {
            self.status = "Choose an export path.".to_string();
            return false;
        }
        match database
            .messages_since(conversation.id, range.start(Local::now()))
            .and_then(|messages| {
                write_markdown(&path, conversation, &range, &messages)?;
                Ok(messages.len())
            }) {
            Ok(count) => {
                self.status = format!("Exported {count} messages to {}", path.display());
                true
            }
            Err(error) => {
                self.status = format!("Export failed: {error:#}");
                false
            }
        }
    }

    fn draw_startup_error(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading("Messages database unavailable");
                ui.add_space(12.0);
                ui.label(
                    "This app needs Full Disk Access to read your local Messages database.\n\
                     Open System Settings > Privacy & Security > Full Disk Access, add this app,\n\
                     enable it, then quit and reopen the app.",
                );
                if let Some(error) = &self.startup_error {
                    ui.add_space(16.0);
                    ui.colored_label(Color32::LIGHT_RED, error);
                }
                ui.add_space(16.0);
                if ui.button("Retry").clicked() {
                    self.connect();
                }
            });
        });
    }

    fn draw_conversations(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("conversations")
            .resizable(true)
            .default_size(320.0)
            .min_size(220.0)
            .show(root, |ui| {
                ui.heading("Conversations");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search contact or phone number")
                            .desired_width(f32::INFINITY),
                    );
                    if !self.search.is_empty() && ui.button("Clear").clicked() {
                        self.search.clear();
                    }
                });
                ui.separator();

                let indices = self.filtered_indices();
                let mut open = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for index in indices {
                        let conversation = &self.conversations[index];
                        let label = format!(
                            "{}\n{}",
                            conversation.name,
                            conversation.last_date.format("%b %-d, %-I:%M %p")
                        );
                        if ui
                            .selectable_label(self.selected == Some(index), label)
                            .clicked()
                        {
                            open = Some(index);
                        }
                    }
                });
                if let Some(index) = open {
                    self.select_conversation(index);
                }
            });
    }

    fn draw_messages(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new(&self.status).small().color(Color32::GRAY));
                });
            });

        egui::CentralPanel::default().show(root, |ui| {
            let Some(index) = self.selected else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a conversation to view its messages.");
                });
                return;
            };
            let conversation_name = self.conversations[index].name.clone();
            let participants = self.conversations[index].participants.clone();
            ui.horizontal(|ui| {
                ui.heading(conversation_name);
                ui.add_space(8.0);
                if ui.button("Export…").clicked() {
                    self.refresh_export_path();
                    self.export_open = true;
                }
            });
            if !participants.is_empty() {
                ui.label(
                    RichText::new(participants.join(", "))
                        .small()
                        .color(Color32::GRAY),
                );
            }
            ui.separator();
            if !self.loaded_all && ui.button("Load older messages").clicked() {
                self.load_older();
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for message in &self.messages {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&message.sender);
                                ui.label(
                                    RichText::new(
                                        message.date.format("%Y-%m-%d %-I:%M:%S %p").to_string(),
                                    )
                                    .small()
                                    .color(Color32::GRAY),
                                );
                            });
                            ui.label(message.display_body());
                        });
                        ui.add_space(5.0);
                    }
                });
        });
    }

    fn draw_export(&mut self, ctx: &egui::Context) {
        if !self.export_open {
            return;
        }
        let mut open = self.export_open;
        let mut should_export = false;
        let previous_choice = self.export_choice;
        egui::Window::new("Export conversation")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                egui::ComboBox::from_label("Range")
                    .selected_text(self.export_choice.label())
                    .show_ui(ui, |ui| {
                        for choice in ExportChoice::ALL {
                            ui.selectable_value(&mut self.export_choice, choice, choice.label());
                        }
                    });
                if matches!(self.export_choice, ExportChoice::Hours | ExportChoice::Days) {
                    ui.horizontal(|ui| {
                        ui.label("Amount");
                        ui.text_edit_singleline(&mut self.export_amount);
                    });
                }
                ui.label("Markdown file");
                ui.add(egui::TextEdit::singleline(&mut self.export_path).desired_width(500.0));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        should_export = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.export_open = false;
                    }
                });
            });
        if self.export_choice != previous_choice {
            self.refresh_export_path();
        }
        if should_export && self.export() {
            open = false;
        }
        self.export_open = open && self.export_open;
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.database.is_none() {
            self.draw_startup_error(ui);
            return;
        }
        self.draw_conversations(ui);
        self.draw_messages(ui);
        self.draw_export(ui.ctx());
    }
}

fn default_export_path(conversation: &Conversation, range: &ExportRange) -> String {
    let filename = format!(
        "{}-{}-{}.md",
        safe_filename(&conversation.name),
        range.label(),
        Local::now().format("%Y-%m-%d")
    );
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Documents").join(&filename))
        .unwrap_or_else(|| PathBuf::from(filename))
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{ExportChoice, ExportRange};

    #[test]
    fn custom_export_ranges_require_positive_numbers() {
        assert!(matches!(
            ExportChoice::Hours.range("12"),
            Ok(ExportRange::Hours(12))
        ));
        assert!(ExportChoice::Days.range("0").is_err());
        assert!(ExportChoice::Hours.range("later").is_err());
    }
}
