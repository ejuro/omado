use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use anyhow::Result;

// Setup: cargo build --release && ./target/release/omarchy-todo
// Hyprland rule: windowrule = opacity 0.9 0.9, class:^(omarchy-todo)

#[derive(Debug)]
enum KeyAction {
    SaveEdit,
    CancelEdit,
    MoveDown,
    MoveUp,
    GoToBottom,
    GoToTop,
    EditSelected,
    AddNew,
    ToggleSelected,
    DeleteKey,
    CycleFilter,
    ClearSearch,
    ClearDelete,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Done,
}

impl Filter {
    fn next(self) -> Self {
        match self {
            Filter::All => Filter::Active,
            Filter::Active => Filter::Done,
            Filter::Done => Filter::All,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active", 
            Filter::Done => "Done",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Todo {
    text: String,
    done: bool,
}

#[derive(Deserialize)]
struct AlacrittyColors {
    primary: Option<AlacrittyPrimary>,
    normal: Option<AlacrittyNormal>,
}

#[derive(Deserialize)]
struct AlacrittyPrimary {
    background: Option<String>,
    foreground: Option<String>,
}

#[derive(Deserialize)]
struct AlacrittyNormal {
    black: Option<String>,
    red: Option<String>,
    green: Option<String>,
    yellow: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
    cyan: Option<String>,
    white: Option<String>,
}

#[derive(Deserialize)]
struct AlacrittyConfig {
    colors: Option<AlacrittyColors>,
    general: Option<AlacrittyGeneral>,
}

#[derive(Deserialize)]
struct AlacrittyGeneral {
    import: Option<Vec<String>>,
}

struct Theme {
    background: egui::Color32,
    foreground: egui::Color32,
    accent: egui::Color32,
    border: egui::Color32,
    done_color: egui::Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: egui::Color32::from_rgb(26, 27, 38),
            foreground: egui::Color32::from_rgb(205, 214, 244),
            accent: egui::Color32::from_rgb(116, 199, 236),
            border: egui::Color32::from_rgb(88, 91, 112),
            done_color: egui::Color32::from_rgb(166, 173, 200),
        }
    }
}

struct TodoApp {
    todos: Vec<Todo>,
    selected: usize,
    filter: Filter,
    search: String,
    editing: Option<usize>,
    edit_text: String,
    theme: Theme,
    last_theme_check: Instant,
    config_path: Option<PathBuf>,
    storage_path: PathBuf,
    delete_mode: bool,
}

impl TodoApp {
    fn new() -> Self {
        let storage_path = Self::get_storage_path();
        let config_path = Self::get_alacritty_config_path();
        
        let mut app = Self {
            todos: Vec::new(),
            selected: 0,
            filter: Filter::All,
            search: String::new(),
            editing: None,
            edit_text: String::new(),
            theme: Theme::default(),
            last_theme_check: Instant::now(),
            config_path,
            storage_path,
            delete_mode: false,
        };
        
        app.load_todos();
        app.load_theme();
        app
    }
    
    fn get_storage_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("omarchy-todo");
        if let Err(_) = fs::create_dir_all(&path) {
            path = PathBuf::from(".");
        }
        path.push("todo.txt");
        path
    }
    
    fn get_alacritty_config_path() -> Option<PathBuf> {
        let mut path = dirs::config_dir()?;
        path.push("alacritty");
        path.push("alacritty.toml");
        if path.exists() { Some(path) } else { None }
    }
    
    fn load_theme(&mut self) {
        if let Some(ref config_path) = self.config_path.clone() {
            self.load_theme_from_file(config_path);
        }
    }
    
    fn load_theme_from_file(&mut self, config_path: &PathBuf) {
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<AlacrittyConfig>(&content) {
                // Check for imported files first
                if let Some(general) = config.general {
                    if let Some(imports) = general.import {
                        for import_path in imports {
                            let expanded = shellexpand::tilde(&import_path);
                            let import_path = PathBuf::from(expanded.as_ref());
                            if import_path.exists() {
                                self.load_theme_from_file(&import_path);
                            }
                        }
                    }
                }
                
                // Load colors from current file (this will override imported ones)
                if let Some(colors) = config.colors {
                    if let Some(primary) = colors.primary {
                        if let Some(bg) = primary.background {
                            if let Ok(color) = Self::parse_hex_color(&bg) {
                                self.theme.background = color;
                            }
                        }
                        if let Some(fg) = primary.foreground {
                            if let Ok(color) = Self::parse_hex_color(&fg) {
                                self.theme.foreground = color;
                            }
                        }
                    }
                    if let Some(normal) = colors.normal {
                        if let Some(blue) = normal.blue {
                            if let Ok(color) = Self::parse_hex_color(&blue) {
                                self.theme.accent = color;
                            }
                        }
                        if let Some(white) = normal.white {
                            if let Ok(color) = Self::parse_hex_color(&white) {
                                self.theme.border = color;
                            }
                        }
                        if let Some(cyan) = normal.cyan {
                            if let Ok(color) = Self::parse_hex_color(&cyan) {
                                self.theme.done_color = color;
                            }
                        } else if let Some(black) = normal.black {
                            if let Ok(color) = Self::parse_hex_color(&black) {
                                self.theme.done_color = color;
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn parse_hex_color(hex: &str) -> Result<egui::Color32> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(anyhow::anyhow!("Invalid hex color length"));
        }
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;  
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        Ok(egui::Color32::from_rgb(r, g, b))
    }
    
    fn load_todos(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.storage_path) {
            self.todos.clear();
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("[ ] ") {
                    self.todos.push(Todo {
                        text: line[4..].to_string(),
                        done: false,
                    });
                } else if line.starts_with("[x] ") {
                    self.todos.push(Todo {
                        text: line[4..].to_string(),
                        done: true,
                    });
                }
            }
        }
    }
    
    fn save_todos(&self) {
        let mut content = String::new();
        for todo in &self.todos {
            let prefix = if todo.done { "[x]" } else { "[ ]" };
            content.push_str(&format!("{} {}\n", prefix, todo.text));
        }
        let _ = fs::write(&self.storage_path, content);
    }
    
    fn filtered_todos(&self) -> Vec<(usize, &Todo)> {
        self.todos
            .iter()
            .enumerate()
            .filter(|(_, todo)| {
                let matches_filter = match self.filter {
                    Filter::All => true,
                    Filter::Active => !todo.done,
                    Filter::Done => todo.done,
                };
                
                let matches_search = if self.search.is_empty() {
                    true
                } else {
                    todo.text.to_lowercase().contains(&self.search.to_lowercase())
                };
                
                matches_filter && matches_search
            })
            .collect()
    }
    
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let mut actions = Vec::new();
        
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = event {
                    if self.editing.is_some() {
                        match key {
                            egui::Key::Enter => actions.push(KeyAction::SaveEdit),
                            egui::Key::Escape => actions.push(KeyAction::CancelEdit),
                            _ => {}
                        }
                        continue;
                    }
                    
                    match key {
                        egui::Key::J | egui::Key::ArrowDown => actions.push(KeyAction::MoveDown),
                        egui::Key::K | egui::Key::ArrowUp => actions.push(KeyAction::MoveUp),
                        egui::Key::G => {
                            if modifiers.shift {
                                actions.push(KeyAction::GoToBottom);
                            } else {
                                actions.push(KeyAction::GoToTop);
                            }
                        }
                        egui::Key::Enter => actions.push(KeyAction::EditSelected),
                        egui::Key::A => actions.push(KeyAction::AddNew),
                        egui::Key::X => actions.push(KeyAction::ToggleSelected),
                        egui::Key::D => actions.push(KeyAction::DeleteKey),
                        egui::Key::F => actions.push(KeyAction::CycleFilter),
                        egui::Key::Escape => actions.push(KeyAction::ClearSearch),
                        _ => actions.push(KeyAction::ClearDelete),
                    }
                }
            }
        });
        
        for action in actions {
            self.handle_key_action(action);
        }
    }
    
    fn handle_key_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::SaveEdit => {
                if let Some(idx) = self.editing {
                    if !self.edit_text.trim().is_empty() {
                        if idx < self.todos.len() {
                            self.todos[idx].text = self.edit_text.trim().to_string();
                        } else {
                            self.todos.push(Todo {
                                text: self.edit_text.trim().to_string(),
                                done: false,
                            });
                        }
                        self.save_todos();
                    }
                    self.editing = None;
                    self.edit_text.clear();
                }
            }
            KeyAction::CancelEdit => {
                self.editing = None;
                self.edit_text.clear();
            }
            KeyAction::MoveDown => {
                let filtered = self.filtered_todos();
                if !filtered.is_empty() {
                    self.selected = (self.selected + 1).min(filtered.len() - 1);
                }
            }
            KeyAction::MoveUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyAction::GoToBottom => {
                let filtered = self.filtered_todos();
                if !filtered.is_empty() {
                    self.selected = filtered.len() - 1;
                }
            }
            KeyAction::GoToTop => {
                self.selected = 0;
            }
            KeyAction::EditSelected => {
                let filtered = self.filtered_todos();
                if let Some((real_idx, todo)) = filtered.get(self.selected) {
                    let real_idx = *real_idx;
                    let text = todo.text.clone();
                    self.editing = Some(real_idx);
                    self.edit_text = text;
                }
            }
            KeyAction::AddNew => {
                self.editing = Some(self.todos.len());
                self.edit_text.clear();
            }
            KeyAction::ToggleSelected => {
                let filtered = self.filtered_todos();
                if let Some((real_idx, _)) = filtered.get(self.selected) {
                    let real_idx = *real_idx;
                    self.todos[real_idx].done = !self.todos[real_idx].done;
                    self.save_todos();
                }
            }
            KeyAction::DeleteKey => {
                if self.delete_mode {
                    let filtered = self.filtered_todos();
                    if let Some((real_idx, _)) = filtered.get(self.selected) {
                        let real_idx = *real_idx;
                        let filtered_len = filtered.len();
                        self.todos.remove(real_idx);
                        if self.selected >= filtered_len - 1 && self.selected > 0 {
                            self.selected -= 1;
                        }
                        self.save_todos();
                    }
                    self.delete_mode = false;
                } else {
                    self.delete_mode = true;
                }
            }
            KeyAction::CycleFilter => {
                self.filter = self.filter.next();
                self.selected = 0;
            }
            KeyAction::ClearSearch => {
                self.search.clear();
                self.selected = 0;
            }
            KeyAction::ClearDelete => {
                self.delete_mode = false;
            }
        }
    }
    
    fn render_todo_list(&mut self, ui: &mut egui::Ui) {
        let is_adding_new = self.editing == Some(self.todos.len());
        
        // Show new todo input at top if adding
        if is_adding_new {
            ui.horizontal(|ui| {
                let bg_color = self.theme.accent.gamma_multiply(0.3);
                
                let frame = egui::Frame::none()
                    .fill(bg_color)
                    .inner_margin(egui::Margin::same(4.0))
                    .outer_margin(egui::Margin::symmetric(0.0, 1.0));
                    
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label("✨ New:");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.edit_text)
                                .hint_text("Enter new todo...")
                                .desired_width(ui.available_width() - 60.0)
                        );
                        response.request_focus();
                    });
                });
            });
            ui.separator();
        }
        
        let filtered = self.filtered_todos();
        
        if filtered.is_empty() && !is_adding_new {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                let text = if self.search.is_empty() {
                    match self.filter {
                        Filter::All => "No todos yet. Press 'a' to add one!",
                        Filter::Active => "No active todos.",
                        Filter::Done => "No completed todos.",
                    }
                } else {
                    "No matching todos found."
                };
                ui.label(egui::RichText::new(text)
                    .color(self.theme.done_color)
                    .size(14.0));
            });
        } else if !filtered.is_empty() {
            // Collect data first to avoid borrow issues
            let mut todo_data = Vec::new();
            for (i, (real_idx, todo)) in filtered.iter().enumerate() {
                todo_data.push((
                    i,
                    *real_idx,
                    todo.text.clone(),
                    todo.done,
                    i == self.selected,
                    self.editing == Some(*real_idx)
                ));
            }
            
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for (_i, _real_idx, text, done, is_selected, is_editing) in todo_data {
                        ui.horizontal(|ui| {
                            let bg_color = if is_selected {
                                self.theme.accent.gamma_multiply(0.3)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            
                            let frame = egui::Frame::none()
                                .fill(bg_color)
                                .inner_margin(egui::Margin::same(4.0))
                                .outer_margin(egui::Margin::symmetric(0.0, 1.0));
                                
                            frame.show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                
                                if is_editing {
                                    ui.horizontal(|ui| {
                                        ui.label("✏️");
                                        let response = ui.add(
                                            egui::TextEdit::singleline(&mut self.edit_text)
                                                .desired_width(ui.available_width() - 30.0)
                                        );
                                        response.request_focus();
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        let checkbox_text = if done { "[x]" } else { "[ ]" };
                                        let text_color = if done {
                                            // Make completed text dimmer but still readable
                                            self.theme.done_color
                                        } else {
                                            self.theme.foreground
                                        };
                                        
                                        ui.label(egui::RichText::new(checkbox_text)
                                            .color(if done { self.theme.accent } else { self.theme.border })
                                            .monospace());
                                            
                                        let display_text = if done {
                                            format!("~~{}~~", text)
                                        } else {
                                            text
                                        };
                                        
                                        ui.label(egui::RichText::new(display_text)
                                            .color(text_color));
                                    });
                                }
                            });
                        });
                    }
                });
        }
    }
}

impl eframe::App for TodoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Hot-reload theme more frequently and force repaints
        if self.last_theme_check.elapsed() > Duration::from_millis(500) {
            self.load_theme();
            self.last_theme_check = Instant::now();
            ctx.request_repaint(); // Force immediate repaint
        }
        
        // Ensure continuous repaints for theme checking
        ctx.request_repaint_after(Duration::from_millis(500));
        
        self.handle_keyboard(ctx);
        
        // Semi-transparent background
        let mut bg_color = self.theme.background;
        bg_color[3] = (255.0 * 0.85) as u8; // 85% opacity
        
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = bg_color;
        style.visuals.panel_fill = bg_color;
        style.visuals.extreme_bg_color = bg_color;
        style.visuals.faint_bg_color = self.theme.border;
        style.visuals.override_text_color = Some(self.theme.foreground);
        style.visuals.selection.bg_fill = self.theme.accent;
        ctx.set_style(style);
        
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_color))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    
                    // Header
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("OMARCHY TODO")
                            .color(self.theme.accent)
                            .size(18.0)
                            .strong());
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("Filter: {}", self.filter.name()))
                                .color(self.theme.done_color)
                                .size(12.0));
                        });
                    });
                    
                    ui.separator();
                    
                    // Search bar
                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        let search_response = ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("Type to search... (/ to focus, Esc to clear)")
                                .desired_width(ui.available_width() - 20.0)
                        );
                        
                        ctx.input(|i| {
                            if i.key_pressed(egui::Key::Slash) && self.editing.is_none() {
                                search_response.request_focus();
                            }
                        });
                    });
                    
                    ui.separator();
                    
                    // Todo list
                    self.render_todo_list(ui);
                    
                    ui.separator();
                    
                    // Help text
                    ui.horizontal(|ui| {
                        let help_text = if self.editing.is_some() {
                            "Enter: Save | Esc: Cancel"
                        } else {
                            "j/k: Move | Enter: Edit | a: Add | x: Toggle | dd: Delete | f: Filter | /: Search"
                        };
                        
                        ui.label(egui::RichText::new(help_text)
                            .color(self.theme.done_color)
                            .size(10.0));
                    });
                    
                    if self.delete_mode {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Press 'd' again to delete selected item")
                                .color(egui::Color32::from_rgb(255, 100, 100))
                                .size(12.0));
                        });
                    }
                });
            });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 640.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_title("omarchy-todo")
            .with_app_id("omarchy-todo"),
        ..Default::default()
    };
    
    eframe::run_native(
        "omarchy-todo",
        options,
        Box::new(|_cc| Ok(Box::new(TodoApp::new()))),
    )
}