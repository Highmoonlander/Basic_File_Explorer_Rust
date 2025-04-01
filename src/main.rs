use iced::widget::{button, checkbox, column, container, horizontal_rule, row, scrollable, text, text_input};
use iced::{executor, theme, Application, Color, Command, Element, Length, Settings, Theme};
use iced::alignment::Horizontal;
use iced::widget::Space;
use std::fs::{create_dir_all, metadata, remove_dir_all, remove_file, File};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;
use chrono::{DateTime, Local};
use humansize::{format_size, BINARY};

pub fn main() -> iced::Result {
    FileManager::run(Settings {
        window: iced::window::Settings {
            size: (900, 700),
            min_size: Some((600, 400)),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
enum Message {
    FileSelected(PathBuf),
    NavigateUp,
    NavigateHome,
    Refresh,
    CreateNew,
    Delete,
    NameInputChanged(String),
    IsDirectoryToggled(bool),
    ConfirmCreate,
    ConfirmDelete,
    ShowProperties,
    CloseDialog,
    SearchInputChanged(String),
    PerformSearch,
    SortByName,
    SortBySize,
    SortByDate,
}

struct FileManager {
    current_dir: PathBuf,
    home_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected_entry: Option<PathBuf>,
    new_name: String,
    is_directory: bool,
    dialog: DialogState,
    properties: Option<FileProperties>,
    search_query: String,
    sort_mode: SortMode,
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
enum SortMode {
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
    DateAsc,
    DateDesc,
}

#[derive(Debug, Clone)]
enum DialogState {
    None,
    Create,
    Delete,
    Properties,
}

#[derive(Debug, Clone)]
struct FileProperties {
    path: PathBuf,
    file_type: String,
    size: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    permissions: String,
}

impl Application for FileManager {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let home_dir = dirs::home_dir().expect("Could not find home directory");
        
        let manager = FileManager {
            current_dir: home_dir.clone(),
            home_dir: home_dir.clone(),
            entries: Vec::new(),
            selected_entry: None,
            new_name: String::new(),
            is_directory: false,
            dialog: DialogState::None,
            properties: None,
            search_query: String::new(),
            sort_mode: SortMode::NameAsc,
        };
        
        (manager, Command::perform(load_directory(home_dir), |_| Message::Refresh))
    }

    fn title(&self) -> String {
        format!("Modern File Manager - {}", self.current_dir.display())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::FileSelected(path) => {
                self.selected_entry = Some(path.clone());
                
                if path.is_dir() {
                    self.current_dir = path;
                    self.selected_entry = None;
                    return Command::perform(load_directory(self.current_dir.clone()), |_| Message::Refresh);
                } else {
                    let _ = open::that(&path);
                }
                
                Command::none()
            }
            Message::NavigateUp => {
                if let Some(parent) = self.current_dir.parent() {
                    if parent.starts_with(&self.home_dir) || parent == self.home_dir.as_path() {
                        self.current_dir = parent.to_path_buf();
                        self.selected_entry = None;
                        return Command::perform(load_directory(self.current_dir.clone()), |_| Message::Refresh);
                    }
                }
                Command::none()
            }
            Message::NavigateHome => {
                self.current_dir = self.home_dir.clone();
                self.selected_entry = None;
                Command::perform(load_directory(self.current_dir.clone()), |_| Message::Refresh)
            }
            Message::Refresh => {
                self.load_entries();
                Command::none()
            }
            Message::CreateNew => {
                self.dialog = DialogState::Create;
                self.new_name = String::new();
                self.is_directory = false;
                Command::none()
            }
            Message::Delete => {
                if self.selected_entry.is_some() {
                    self.dialog = DialogState::Delete;
                }
                Command::none()
            }
            Message::NameInputChanged(name) => {
                self.new_name = name;
                Command::none()
            }
            Message::IsDirectoryToggled(is_dir) => {
                self.is_directory = is_dir;
                Command::none()
            }
            Message::ConfirmCreate => {
                if !self.new_name.is_empty() {
                    let path = self.current_dir.join(&self.new_name);
                    
                    if self.is_directory {
                        let _ = create_dir_all(&path);
                    } else {
                        let _ = File::create(&path);
                    }
                }
                
                self.dialog = DialogState::None;
                Command::perform(load_directory(self.current_dir.clone()), |_| Message::Refresh)
            }
            Message::ConfirmDelete => {
                if let Some(path) = &self.selected_entry {
                    if path.is_dir() {
                        let _ = remove_dir_all(path);
                    } else {
                        let _ = remove_file(path);
                    }
                    
                    self.selected_entry = None;
                    self.dialog = DialogState::None;
                    return Command::perform(load_directory(self.current_dir.clone()), |_| Message::Refresh);
                }
                Command::none()
            }
            Message::ShowProperties => {
                if let Some(path) = &self.selected_entry {
                    if let Ok(meta) = metadata(path) {
                        let permissions = if cfg!(unix) {
                            use std::os::unix::fs::PermissionsExt;
                            format!("{:o}", meta.permissions().mode() & 0o777)
                        } else {
                            if meta.permissions().readonly() {
                                "Read-only".to_string()
                            } else {
                                "Read-write".to_string()
                            }
                        };
                        
                        self.properties = Some(FileProperties {
                            path: path.clone(),
                            file_type: if path.is_dir() { "Directory".to_string() } else { "File".to_string() },
                            size: meta.len(),
                            modified: meta.modified().unwrap_or(SystemTime::now()),
                            created: meta.created().ok(),
                            permissions,
                        });
                        
                        self.dialog = DialogState::Properties;
                    }
                }
                Command::none()
            }
            Message::CloseDialog => {
                self.dialog = DialogState::None;
                Command::none()
            }
            Message::SearchInputChanged(query) => {
                self.search_query = query;
                Command::none()
            }
            Message::PerformSearch => {
                self.load_entries();
                Command::none()
            }
            Message::SortByName => {
                self.sort_mode = if self.sort_mode == SortMode::NameAsc {
                    SortMode::NameDesc
                } else {
                    SortMode::NameAsc
                };
                self.sort_entries();
                Command::none()
            }
            Message::SortBySize => {
                self.sort_mode = if self.sort_mode == SortMode::SizeAsc {
                    SortMode::SizeDesc
                } else {
                    SortMode::SizeAsc
                };
                self.sort_entries();
                Command::none()
            }
            Message::SortByDate => {
                self.sort_mode = if self.sort_mode == SortMode::DateAsc {
                    SortMode::DateDesc
                } else {
                    SortMode::DateAsc
                };
                self.sort_entries();
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let title = text(format!("Current Directory: {}", self.current_dir.display()))
            .size(20)
            .width(Length::Fill);

        // Navigation buttons with improved styling
        let nav_button = button(
            row![text("⬆️ Up").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::NavigateUp)
        .padding(10)
        .width(Length::Fill)
        .style(theme::Button::Secondary);

        let home_button = button(
            row![text("🏠 Home").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::NavigateHome)
        .padding(10)
        .width(Length::Fill)
        .style(theme::Button::Secondary);

        let refresh_button = button(
            row![text("🔄 Refresh").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::Refresh)
        .padding(10)
        .width(Length::Fill)
        .style(theme::Button::Secondary);

        // Action buttons with improved styling
        let create_button = button(
            row![text("➕ New").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::CreateNew)
        .padding(10)
        .width(Length::Fill)
        .style(theme::Button::Primary);

        let delete_button = button(
            row![text("🗑️ Delete").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::Delete)
        .padding(10)
        .width(Length::Fill)
        .style(if self.selected_entry.is_some() {
            theme::Button::Destructive
        } else {
            theme::Button::Secondary
        });

        let properties_button = button(
            row![text("ℹ️ Properties").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::ShowProperties)
        .padding(10)
        .width(Length::Fill)
        .style(if self.selected_entry.is_some() {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        });

        // Search bar
        let search_input = text_input("Search files...", &self.search_query)
            .on_input(Message::SearchInputChanged)
            .on_submit(Message::PerformSearch)
            .padding(10);

        let search_button = button(
            row![text("🔍 Search").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::PerformSearch)
        .padding(10)
        .width(Length::Fixed(100.0))
        .style(theme::Button::Secondary);

        let search_row = row![search_input, search_button]
            .spacing(10)
            .padding(10);

        // Navigation controls
        let nav_controls = row![nav_button, home_button, refresh_button]
            .spacing(10)
            .padding(10);

        // Action controls
        let action_controls = row![create_button, delete_button, properties_button]
            .spacing(10)
            .padding(10);

        // Sort buttons
        let sort_name_button = button(
            row![text("Sort by Name").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::SortByName)
        .padding(5)
        .width(Length::Fill)
        .style(if matches!(self.sort_mode, SortMode::NameAsc | SortMode::NameDesc) {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        });

        let sort_size_button = button(
            row![text("Sort by Size").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::SortBySize)
        .padding(5)
        .width(Length::Fill)
        .style(if matches!(self.sort_mode, SortMode::SizeAsc | SortMode::SizeDesc) {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        });

        let sort_date_button = button(
            row![text("Sort by Date").horizontal_alignment(Horizontal::Center)]
                .width(Length::Fill)
                .align_items(iced::Alignment::Center)
        )
        .on_press(Message::SortByDate)
        .padding(5)
        .width(Length::Fill)
        .style(if matches!(self.sort_mode, SortMode::DateAsc | SortMode::DateDesc) {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        });

        let sort_controls = row![sort_name_button, sort_size_button, sort_date_button]
            .spacing(10)
            .padding(5);

        // File list header
        let header_row = row![
            text("Name").width(Length::FillPortion(3)),
            text("Size").width(Length::FillPortion(1)),
            text("Modified").width(Length::FillPortion(2))
        ]
        .padding(10)
        .spacing(10);

        // File list with improved styling
        let file_list = self.entries.iter().fold(
            column![header_row].spacing(2),
            |column, entry| {
                let path = &entry.path;
                let is_selected = self
                    .selected_entry
                    .as_ref()
                    .map_or(false, |selected| selected == path);
                
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");
                
                let icon = if path.is_dir() { "📁 " } else { "📄 " };
                
                let size_text = if path.is_dir() {
                    "Folder".to_string()
                } else {
                    format_size(entry.size, BINARY)
                };
                
                let modified: DateTime<Local> = entry.modified.into();
                let date_text = modified.format("%Y-%m-%d %H:%M").to_string();
                
                let file_row = row![
                    text(format!("{}{}", icon, name)).width(Length::FillPortion(3)),
                    text(size_text).width(Length::FillPortion(1)),
                    text(date_text).width(Length::FillPortion(2))
                ]
                .spacing(10)
                .padding(10)
                .width(Length::Fill);
                
                let file_button = button(file_row)
                    .width(Length::Fill)
                    .on_press(Message::FileSelected(path.clone()))
                    .style(if is_selected {
                        theme::Button::Primary
                    } else {
                        theme::Button::Text
                    });
                
                column.push(file_button)
            },
        );

        // Create scrollable with updated API
        let files_scrollable = scrollable(file_list)
            .height(Length::Fill)
            .width(Length::Fill);

        // Status bar showing item count
        let status_bar = container(
            text(format!("{} items", self.entries.len()))
                .size(14)
        )
        .width(Length::Fill)
        .padding(5)
        .style(theme::Container::Box);

        // Main content layout
        let content = column![
            title,
            search_row,
            row![
                column![nav_controls].width(Length::FillPortion(1)),
                column![action_controls].width(Length::FillPortion(1))
            ],
            sort_controls,
            horizontal_rule(1),
            files_scrollable,
            status_bar
        ]
        .spacing(5)
        .padding(20);

        // Main container
        let main_content = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Box);

        // If we have a dialog active, create an overlay
        match &self.dialog {
            DialogState::None => main_content.into(),
            DialogState::Create => self.create_dialog(),
            DialogState::Delete => self.delete_dialog(),
            DialogState::Properties => self.properties_dialog(),
        }
    }
}

// Helper methods for FileManager
impl FileManager {
    fn load_entries(&mut self) {
        self.entries.clear();
        
        for entry in WalkDir::new(&self.current_dir).max_depth(1) {
            if let Ok(entry) = entry {
                let path = entry.path().to_path_buf();
                
                // Skip the current directory
                if path == self.current_dir {
                    continue;
                }
                
                // Skip hidden files unless explicitly searching for them
                if is_hidden(&path) && !self.search_query.starts_with('.') {
                    continue;
                }
                
                // Apply search filter if query is not empty
                if !self.search_query.is_empty() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !name.to_lowercase().contains(&self.search_query.to_lowercase()) {
                            continue;
                        }
                    }
                }
                
                // Get file metadata
                if let Ok(meta) = metadata(&path) {
                    self.entries.push(FileEntry {
                        path,
                        size: meta.len(),
                        modified: meta.modified().unwrap_or(SystemTime::now()),
                    });
                } else {
                    // If metadata can't be read, still show the file with default values
                    self.entries.push(FileEntry {
                        path,
                        size: 0,
                        modified: SystemTime::now(),
                    });
                }
            }
        }
        
        self.sort_entries();
    }
    
    fn sort_entries(&mut self) {
        match self.sort_mode {
            SortMode::NameAsc => {
                // Sort directories first, then files alphabetically
                self.entries.sort_by(|a, b| {
                    let a_is_dir = a.path.is_dir();
                    let b_is_dir = b.path.is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.path.file_name().cmp(&b.path.file_name()),
                    }
                });
            },
            SortMode::NameDesc => {
                // Sort directories first, then files reverse alphabetically
                self.entries.sort_by(|a, b| {
                    let a_is_dir = a.path.is_dir();
                    let b_is_dir = b.path.is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => b.path.file_name().cmp(&a.path.file_name()),
                    }
                });
            },
            SortMode::SizeAsc => {
                // Sort by size (ascending)
                self.entries.sort_by(|a, b| {
                    let a_is_dir = a.path.is_dir();
                    let b_is_dir = b.path.is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, true) => a.path.file_name().cmp(&b.path.file_name()),
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        (false, false) => a.size.cmp(&b.size),
                    }
                });
            },
            SortMode::SizeDesc => {
                // Sort by size (descending)
                self.entries.sort_by(|a, b| {
                    let a_is_dir = a.path.is_dir();
                    let b_is_dir = b.path.is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, true) => a.path.file_name().cmp(&b.path.file_name()),
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        (false, false) => b.size.cmp(&a.size),
                    }
                });
            },
            SortMode::DateAsc => {
                // Sort by modification date (ascending)
                self.entries.sort_by(|a, b| {
                    let a_is_dir = a.path.is_dir();
                    let b_is_dir = b.path.is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.modified.cmp(&b.modified),
                    }
                });
            },
            SortMode::DateDesc => {
                // Sort by modification date (descending)
                self.entries.sort_by(|a, b| {
                    let a_is_dir = a.path.is_dir();
                    let b_is_dir = b.path.is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => b.modified.cmp(&a.modified),
                    }
                });
            },
        }
    }

    fn create_dialog<'a>(&self) -> Element<'a, Message> {
        // Create a semi-transparent overlay
        let overlay = container(
            // Dialog content
            container(
                column![
                    text("Create New").size(24),
                    Space::with_height(Length::Fixed(10.0)),
                    text_input("Enter name...", &self.new_name)
                        .on_input(Message::NameInputChanged)
                        .padding(10),
                    row![
                        checkbox("Is Directory", self.is_directory, Message::IsDirectoryToggled)
                    ]
                    .padding(10),
                    Space::with_height(Length::Fixed(10.0)),
                    row![
                        button(text("Cancel").horizontal_alignment(Horizontal::Center))
                            .on_press(Message::CloseDialog)
                            .padding(10)
                            .width(Length::Fixed(100.0))
                            .style(theme::Button::Secondary),
                        button(text("Create").horizontal_alignment(Horizontal::Center))
                            .on_press(Message::ConfirmCreate)
                            .padding(10)
                            .width(Length::Fixed(100.0))
                            .style(theme::Button::Primary)
                    ]
                    .spacing(10)
                    .align_items(iced::Alignment::Center)
                ]
                .spacing(20)
                .padding(20)
                .width(Length::Fixed(400.0))
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fixed(400.0))
            .padding(20)
            .center_x()
            .center_y()
            .style(theme::Container::Box)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(theme::Container::Box);

        overlay.into()
    }

    fn delete_dialog<'a>(&self) -> Element<'a, Message> {
        let name = self
            .selected_entry
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("this item");

        // Create a semi-transparent overlay
        let overlay = container(
            // Dialog content
            container(
                column![
                    text(format!("Delete '{}'?", name)).size(24),
                    Space::with_height(Length::Fixed(10.0)),
                    text("This action cannot be undone.").size(16),
                    Space::with_height(Length::Fixed(20.0)),
                    row![
                        button(text("Cancel").horizontal_alignment(Horizontal::Center))
                            .on_press(Message::CloseDialog)
                            .padding(10)
                            .width(Length::Fixed(100.0))
                            .style(theme::Button::Secondary),
                        button(text("Delete").horizontal_alignment(Horizontal::Center))
                            .on_press(Message::ConfirmDelete)
                            .padding(10)
                            .width(Length::Fixed(100.0))
                            .style(theme::Button::Destructive)
                    ]
                    .spacing(10)
                    .align_items(iced::Alignment::Center)
                ]
                .spacing(20)
                .padding(20)
                .width(Length::Fixed(400.0))
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fixed(400.0))
            .padding(20)
            .center_x()
            .center_y()
            .style(theme::Container::Box)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(theme::Container::Box);

        overlay.into()
    }

    fn properties_dialog<'a>(&self) -> Element<'a, Message> {
        let properties = if let Some(props) = &self.properties {
            let modified: DateTime<Local> = props.modified.into();
            
            let created_text = if let Some(created) = props.created {
                let created: DateTime<Local> = created.into();
                created.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                "Unknown".to_string()
            };
            
            column![
                row![
                    text("Path:").width(Length::Fixed(100.0)),
                    text(format!("{}", props.path.display())).width(Length::Fill)
                ].padding(5),
                row![
                    text("Type:").width(Length::Fixed(100.0)),
                    text(format!("{}", props.file_type)).width(Length::Fill)
                ].padding(5),
                row![
                    text("Size:").width(Length::Fixed(100.0)),
                    text(format!("{}", format_size(props.size, BINARY))).width(Length::Fill)
                ].padding(5),
                row![
                    text("Modified:").width(Length::Fixed(100.0)),
                    text(format!("{}", modified.format("%Y-%m-%d %H:%M:%S"))).width(Length::Fill)
                ].padding(5),
                row![
                    text("Created:").width(Length::Fixed(100.0)),
                    text(created_text).width(Length::Fill)
                ].padding(5),
                row![
                    text("Permissions:").width(Length::Fixed(100.0)),
                    text(format!("{}", props.permissions)).width(Length::Fill)
                ].padding(5),
            ]
        } else {
            column![text("No properties available").size(16)]
        };

        // Create a semi-transparent overlay
        let overlay = container(
            // Dialog content
            container(
                column![
                    text("File Properties").size(24),
                    Space::with_height(Length::Fixed(20.0)),
                    properties,
                    Space::with_height(Length::Fixed(20.0)),
                    button(text("Close").horizontal_alignment(Horizontal::Center))
                        .on_press(Message::CloseDialog)
                        .padding(10)
                        .width(Length::Fixed(100.0))
                        .style(theme::Button::Secondary)
                ]
                .spacing(10)
                .padding(20)
                .width(Length::Fixed(500.0))
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fixed(500.0))
            .padding(20)
            .center_x()
            .center_y()
            .style(theme::Container::Box)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(theme::Container::Box);

        overlay.into()
    }
}

async fn load_directory(_path: PathBuf) -> () {
    // This is a fake async function to make the Command happy
    // The actual loading happens in load_entries
    ()
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}