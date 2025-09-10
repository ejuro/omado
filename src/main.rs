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
    OpenProjectPalette,
    ToggleSearch,
    CycleProject,
    ClearAllFilters,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Done,
}

#[derive(Clone, PartialEq)]
enum ProjectFilter {
    All,
    NoProject,
    Project(String),
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
    project: Option<String>,
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
    #[allow(dead_code)]
    red: Option<String>,
    #[allow(dead_code)]
    green: Option<String>,
    #[allow(dead_code)]
    yellow: Option<String>,
    blue: Option<String>,
    #[allow(dead_code)]
    magenta: Option<String>,
    cyan: Option<String>,
    white: Option<String>,
}

#[derive(Deserialize)]
struct AlacrittyConfig {
    colors: Option<AlacrittyColors>,
    general: Option<AlacrittyGeneral>,
    font: Option<AlacrittyFont>,
}

#[derive(Deserialize)]
struct AlacrittyFont {
    normal: Option<AlacrittyFontFamily>,
    size: Option<f32>,
}

#[derive(Deserialize)]
struct AlacrittyFontFamily {
    family: Option<String>,
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
    // Additional colors from Alacritty theme for project coloring
    red: Option<egui::Color32>,
    green: Option<egui::Color32>,
    yellow: Option<egui::Color32>,
    blue: Option<egui::Color32>,
    magenta: Option<egui::Color32>,
    cyan: Option<egui::Color32>,
    white: Option<egui::Color32>,
    font_family: Option<String>,
    font_size: Option<f32>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: egui::Color32::from_rgb(26, 27, 38),
            foreground: egui::Color32::from_rgb(205, 214, 244),
            accent: egui::Color32::from_rgb(116, 199, 236),
            border: egui::Color32::from_rgb(88, 91, 112),
            done_color: egui::Color32::from_rgb(166, 173, 200),
            // Default Catppuccin-like colors for projects
            red: Some(egui::Color32::from_rgb(243, 139, 168)),
            green: Some(egui::Color32::from_rgb(166, 227, 161)),
            yellow: Some(egui::Color32::from_rgb(249, 226, 175)),
            blue: Some(egui::Color32::from_rgb(137, 180, 250)),
            magenta: Some(egui::Color32::from_rgb(203, 166, 247)),
            cyan: Some(egui::Color32::from_rgb(148, 226, 213)),
            white: Some(egui::Color32::from_rgb(205, 214, 244)),
            font_family: None,
            font_size: None,
        }
    }
}

struct TodoApp {
    todos: Vec<Todo>,
    selected: usize,
    filter: Filter,
    project_filter: ProjectFilter,
    search: String,
    editing: Option<usize>,
    edit_text: String,
    theme: Theme,
    last_theme_check: Instant,
    config_path: Option<PathBuf>,
    storage_path: PathBuf,
    delete_mode: bool,
    show_project_palette: bool,
    project_palette_search: String,
    project_palette_selected: usize,
    show_search: bool,
    user_font_size: Option<f32>,
}

impl TodoApp {
    fn new() -> Self {
        let storage_path = Self::get_storage_path();
        let config_path = Self::get_alacritty_config_path();
        
        let mut app = Self {
            todos: Vec::new(),
            selected: 0,
            filter: Filter::All,
            project_filter: ProjectFilter::All,
            search: String::new(),
            editing: None,
            edit_text: String::new(),
            theme: Theme::default(),
            last_theme_check: Instant::now(),
            config_path,
            storage_path,
            delete_mode: false,
            show_project_palette: false,
            project_palette_search: String::new(),
            project_palette_selected: 0,
            show_search: false,
            user_font_size: None,
        };
        
        app.load_todos();
        app.load_theme();
        app
    }
    
    pub fn get_storage_path() -> PathBuf {
        // Use XDG_DATA_HOME or fallback to ~/.local/share for Linux
        let data_dir = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
                home.push(".local/share");
                home
            });
        
        let mut path = data_dir;
        path.push("omado");
        if let Err(_) = fs::create_dir_all(&path) {
            path = PathBuf::from(".");
        }
        path.push("todo.txt");
        path
    }
    
    fn get_alacritty_config_path() -> Option<PathBuf> {
        // Use XDG_CONFIG_HOME or fallback to ~/.config for Linux
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path
        } else {
            return None;
        };
        
        let mut path = config_dir;
        path.push("alacritty");
        path.push("alacritty.toml");
        if path.exists() { Some(path) } else { None }
    }
    
    fn load_theme(&mut self) {
        // Reset to default theme first to ensure clean state
        self.theme = Theme::default();
        
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
                                self.theme.blue = Some(color);
                            }
                        }
                        if let Some(white) = normal.white {
                            if let Ok(color) = Self::parse_hex_color(&white) {
                                self.theme.border = color;
                                self.theme.white = Some(color);
                            }
                        }
                        if let Some(cyan) = normal.cyan {
                            if let Ok(color) = Self::parse_hex_color(&cyan) {
                                self.theme.done_color = color;
                                self.theme.cyan = Some(color);
                            }
                        } else if let Some(black) = normal.black {
                            if let Ok(color) = Self::parse_hex_color(&black) {
                                self.theme.done_color = color;
                            }
                        }
                        
                        // Load additional colors for project names
                        if let Some(red) = normal.red {
                            if let Ok(color) = Self::parse_hex_color(&red) {
                                self.theme.red = Some(color);
                            }
                        }
                        if let Some(green) = normal.green {
                            if let Ok(color) = Self::parse_hex_color(&green) {
                                self.theme.green = Some(color);
                            }
                        }
                        if let Some(yellow) = normal.yellow {
                            if let Ok(color) = Self::parse_hex_color(&yellow) {
                                self.theme.yellow = Some(color);
                            }
                        }
                        if let Some(magenta) = normal.magenta {
                            if let Ok(color) = Self::parse_hex_color(&magenta) {
                                self.theme.magenta = Some(color);
                            }
                        }
                    }
                }
                
                // Load font settings
                if let Some(font) = config.font {
                    if let Some(size) = font.size {
                        self.theme.font_size = Some(size);
                    }
                    if let Some(normal) = font.normal {
                        if let Some(family) = normal.family {
                            self.theme.font_family = Some(family);
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
    
    pub fn parse_todo_text(text: &str) -> (String, Option<String>) {
        if let Some(colon_pos) = text.find(':') {
            let project_part = &text[..colon_pos].trim();
            let task_part = &text[colon_pos + 1..].trim();
            if !project_part.is_empty() && !task_part.is_empty() {
                return (task_part.to_string(), Some(project_part.to_string()));
            }
        }
        (text.to_string(), None)
    }
    
    fn load_todos(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.storage_path) {
            self.todos.clear();
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("[ ] ") {
                    let (text, project) = Self::parse_todo_text(&line[4..]);
                    self.todos.push(Todo {
                        text,
                        done: false,
                        project,
                    });
                } else if line.starts_with("[x] ") {
                    let (text, project) = Self::parse_todo_text(&line[4..]);
                    self.todos.push(Todo {
                        text,
                        done: true,
                        project,
                    });
                }
            }
        }
    }
    
    fn save_todos(&self) {
        let mut content = String::new();
        for todo in &self.todos {
            let prefix = if todo.done { "[x]" } else { "[ ]" };
            let display_text = if let Some(ref project) = todo.project {
                format!("{}: {}", project, todo.text)
            } else {
                todo.text.clone()
            };
            content.push_str(&format!("{} {}\n", prefix, display_text));
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
                
                let matches_project = match &self.project_filter {
                    ProjectFilter::All => true,
                    ProjectFilter::NoProject => todo.project.is_none(),
                    ProjectFilter::Project(project) => {
                        todo.project.as_ref() == Some(project)
                    },
                };
                
                let matches_search = if self.search.is_empty() {
                    true
                } else {
                    let search_lower = self.search.to_lowercase();
                    todo.text.to_lowercase().contains(&search_lower) ||
                    todo.project.as_ref().map_or(false, |p| p.to_lowercase().contains(&search_lower))
                };
                
                matches_filter && matches_project && matches_search
            })
            .collect()
    }
    
    fn get_all_projects(&self) -> Vec<String> {
        let mut projects: Vec<String> = self.todos
            .iter()
            .filter_map(|todo| todo.project.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        projects.sort();
        projects
    }
    
    fn get_project_task_count(&self, project: Option<&String>) -> (usize, usize) {
        let todos_for_project: Vec<_> = self.todos
            .iter()
            .filter(|todo| {
                match project {
                    Some(p) => todo.project.as_ref() == Some(p),
                    None => todo.project.is_none(),
                }
            })
            .collect();
        
        let total = todos_for_project.len();
        let active = todos_for_project.iter().filter(|todo| !todo.done).count();
        (active, total)
    }
    
    fn get_project_color(&self, project: &str) -> egui::Color32 {
        // Use the most contrasting colors from the current theme
        // Prioritize accent and bright colors that contrast well with foreground text
        let mut project_colors = Vec::new();
        
        // Always include accent color as it's designed to stand out
        project_colors.push(self.theme.accent);
        
        // Add theme colors that contrast well with foreground text
        if let Some(red) = self.theme.red {
            project_colors.push(red);
        }
        if let Some(green) = self.theme.green {
            project_colors.push(green);
        }
        if let Some(yellow) = self.theme.yellow {
            project_colors.push(yellow);
        }
        if let Some(blue) = self.theme.blue {
            // Only add blue if it's different from accent (since accent often uses blue)
            if blue != self.theme.accent {
                project_colors.push(blue);
            }
        }
        if let Some(magenta) = self.theme.magenta {
            project_colors.push(magenta);
        }
        if let Some(cyan) = self.theme.cyan {
            // Only add cyan if it's different from done_color
            if cyan != self.theme.done_color {
                project_colors.push(cyan);
            }
        }
        
        // If we don't have enough colors, create brighter variants of existing ones
        if project_colors.len() < 4 {
            let brightened_accent = egui::Color32::from_rgb(
                (self.theme.accent.r() as u16 * 120 / 100).min(255) as u8,
                (self.theme.accent.g() as u16 * 120 / 100).min(255) as u8,
                (self.theme.accent.b() as u16 * 120 / 100).min(255) as u8,
            );
            project_colors.push(brightened_accent);
            
            let brightened_border = egui::Color32::from_rgb(
                (self.theme.border.r() as u16 * 140 / 100).min(255) as u8,
                (self.theme.border.g() as u16 * 140 / 100).min(255) as u8,
                (self.theme.border.b() as u16 * 140 / 100).min(255) as u8,
            );
            project_colors.push(brightened_border);
        }
        
        // Simple hash function to consistently map project names to colors
        let mut hash: u32 = 0;
        for byte in project.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        
        let color_index = (hash as usize) % project_colors.len();
        project_colors[color_index]
    }
    
    fn render_project_palette(&mut self, ctx: &egui::Context) {
        if !self.show_project_palette {
            return;
        }
        
        egui::Window::new("Project Palette")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    
                    // Search input
                    ui.horizontal(|ui| {
                        ui.label("🔍");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.project_palette_search)
                                .hint_text("Filter projects...")
                                .desired_width(ui.available_width() - 30.0)
                        );
                        response.request_focus();
                    });
                    
                    ui.separator();
                    
                    // Build list of project options
                    let mut options = vec![("All".to_string(), None, self.todos.len())];
                    
                    let (_no_project_active, no_project_total) = self.get_project_task_count(None);
                    options.push(("No project".to_string(), Some(None), no_project_total));
                    
                    let projects = self.get_all_projects();
                    for project in projects {
                        let (_active_count, total_count) = self.get_project_task_count(Some(&project));
                        options.push((project.clone(), Some(Some(project)), total_count));
                    }
                    
                    // Filter options based on search
                    let filtered_options: Vec<_> = options
                        .into_iter()
                        .filter(|(name, _, _)| {
                            if self.project_palette_search.is_empty() {
                                true
                            } else {
                                name.to_lowercase().contains(&self.project_palette_search.to_lowercase())
                            }
                        })
                        .collect();
                    
                    // Adjust selection if it's out of bounds
                    if self.project_palette_selected >= filtered_options.len() {
                        self.project_palette_selected = filtered_options.len().saturating_sub(1);
                    }
                    
                    // Handle keyboard input
                    ctx.input(|i| {
                        for event in &i.events {
                            if let egui::Event::Key { key, pressed: true, .. } = event {
                                match key {
                                    egui::Key::ArrowDown | egui::Key::J => {
                                        if self.project_palette_selected < filtered_options.len().saturating_sub(1) {
                                            self.project_palette_selected += 1;
                                        }
                                    }
                                    egui::Key::ArrowUp | egui::Key::K => {
                                        if self.project_palette_selected > 0 {
                                            self.project_palette_selected -= 1;
                                        }
                                    }
                                    egui::Key::Enter => {
                                        if let Some((_, filter_option, _)) = filtered_options.get(self.project_palette_selected) {
                                            match filter_option {
                                                None => self.project_filter = ProjectFilter::All,
                                                Some(None) => self.project_filter = ProjectFilter::NoProject,
                                                Some(Some(project)) => self.project_filter = ProjectFilter::Project(project.clone()),
                                            }
                                            self.show_project_palette = false;
                                            self.selected = 0;
                                        }
                                    }
                                    egui::Key::Escape => {
                                        self.show_project_palette = false;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    });
                    
                    // Render options
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for (i, (name, filter_option, _total_count)) in filtered_options.iter().enumerate() {
                                let is_selected = i == self.project_palette_selected;
                                let bg_color = if is_selected {
                                    self.theme.accent.gamma_multiply(0.3)
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                
                                let frame = egui::Frame::none()
                                    .fill(bg_color)
                                    .inner_margin(egui::Margin::same(4.0));
                                
                                frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(if is_selected { "▶" } else { " " });
                                        ui.label(name);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let active_count = match filter_option {
                                                None => {
                                                    let active = self.todos.iter().filter(|t| !t.done).count();
                                                    active
                                                },
                                                Some(None) => {
                                                    let (active, _) = self.get_project_task_count(None);
                                                    active
                                                },
                                                Some(Some(project)) => {
                                                    let (active, _) = self.get_project_task_count(Some(project));
                                                    active
                                                },
                                            };
                                            ui.label(format!("{} open", active_count));
                                        });
                                    });
                                });
                            }
                        });
                    
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("j/k: Move | Enter: Select | Esc: Cancel")
                            .color(self.theme.done_color)
                            .size(10.0));
                    });
                });
            });
    }
    
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let mut actions = Vec::new();
        
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = event {
                    // Skip main keyboard handling if project palette is open
                    // Allow Escape key through even if search is open
                    if self.show_project_palette || (self.show_search && !matches!(key, egui::Key::Escape)) {
                        continue;
                    }
                    
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
                        egui::Key::P => {
                            if modifiers.shift {
                                actions.push(KeyAction::OpenProjectPalette);
                            } else {
                                actions.push(KeyAction::CycleProject);
                            }
                        }
                        egui::Key::S => {
                            if modifiers.shift {
                                actions.push(KeyAction::ToggleSearch);
                            } else {
                                actions.push(KeyAction::ClearDelete);
                            }
                        }
                        egui::Key::Enter => actions.push(KeyAction::EditSelected),
                        egui::Key::A => actions.push(KeyAction::AddNew),
                        egui::Key::X => actions.push(KeyAction::ToggleSelected),
                        egui::Key::D => actions.push(KeyAction::DeleteKey),
                        egui::Key::F => actions.push(KeyAction::CycleFilter),
                        egui::Key::C => actions.push(KeyAction::ClearAllFilters),
                        egui::Key::Plus | egui::Key::Equals => {
                            if modifiers.ctrl {
                                actions.push(KeyAction::IncreaseFontSize);
                            } else {
                                actions.push(KeyAction::ClearDelete);
                            }
                        }
                        egui::Key::Minus => {
                            if modifiers.ctrl {
                                actions.push(KeyAction::DecreaseFontSize);
                            } else {
                                actions.push(KeyAction::ClearDelete);
                            }
                        }
                        egui::Key::Num0 => {
                            if modifiers.ctrl {
                                actions.push(KeyAction::ResetFontSize);
                            } else {
                                actions.push(KeyAction::ClearDelete);
                            }
                        }
                        egui::Key::Slash => {}, // Handle search focus separately to avoid conflicts
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
                        let (text, project) = Self::parse_todo_text(self.edit_text.trim());
                        if idx < self.todos.len() {
                            self.todos[idx].text = text;
                            self.todos[idx].project = project;
                        } else {
                            self.todos.push(Todo {
                                text,
                                done: false,
                                project,
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
                    let text = if let Some(ref project) = todo.project {
                        format!("{}: {}", project, todo.text)
                    } else {
                        todo.text.clone()
                    };
                    self.editing = Some(real_idx);
                    self.edit_text = text;
                }
            }
            KeyAction::AddNew => {
                self.editing = Some(self.todos.len());
                // Pre-fill with current project if one is selected
                self.edit_text = match &self.project_filter {
                    ProjectFilter::Project(project) => format!("{}: ", project),
                    _ => String::new(),
                };
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
                self.show_search = false;
                self.selected = 0;
            }
            KeyAction::ClearDelete => {
                self.delete_mode = false;
            }
            KeyAction::OpenProjectPalette => {
                self.show_project_palette = true;
                self.project_palette_search.clear();
                self.project_palette_selected = 0;
            }
            KeyAction::ToggleSearch => {
                self.show_search = !self.show_search;
                if !self.show_search {
                    self.search.clear();
                    self.selected = 0;
                }
            }
            KeyAction::CycleProject => {
                let projects = self.get_all_projects();
                match &self.project_filter {
                    ProjectFilter::All => {
                        self.project_filter = ProjectFilter::NoProject;
                    }
                    ProjectFilter::NoProject => {
                        if let Some(first_project) = projects.first() {
                            self.project_filter = ProjectFilter::Project(first_project.clone());
                        } else {
                            self.project_filter = ProjectFilter::All;
                        }
                    }
                    ProjectFilter::Project(current) => {
                        if let Some(current_idx) = projects.iter().position(|p| p == current) {
                            let next_idx = current_idx + 1;
                            if next_idx < projects.len() {
                                self.project_filter = ProjectFilter::Project(projects[next_idx].clone());
                            } else {
                                // After the last project, go back to All
                                self.project_filter = ProjectFilter::All;
                            }
                        } else {
                            // Current project not found, go to All
                            self.project_filter = ProjectFilter::All;
                        }
                    }
                }
                self.selected = 0;
            }
            KeyAction::ClearAllFilters => {
                self.filter = Filter::All;
                self.project_filter = ProjectFilter::All;
                self.search.clear();
                self.show_search = false;
                self.selected = 0;
            }
            KeyAction::IncreaseFontSize => {
                let current_size = self.user_font_size
                    .or(self.theme.font_size)
                    .unwrap_or(14.0);
                self.user_font_size = Some((current_size + 1.0).min(24.0));
            }
            KeyAction::DecreaseFontSize => {
                let current_size = self.user_font_size
                    .or(self.theme.font_size)
                    .unwrap_or(14.0);
                self.user_font_size = Some((current_size - 1.0).max(8.0));
            }
            KeyAction::ResetFontSize => {
                self.user_font_size = None;
            }
        }
    }
    
    fn get_effective_font_size(&self) -> f32 {
        self.user_font_size
            .or(self.theme.font_size)
            .unwrap_or(14.0)
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
                    todo.project.clone(),
                    todo.done,
                    i == self.selected,
                    self.editing == Some(*real_idx)
                ));
            }
            
            let _scroll_area = egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(ui.available_height() - 100.0) // Leave space for help text
                .show(ui, |ui| {
                    for (_i, _real_idx, text, project, done, is_selected, is_editing) in todo_data {
                        
                        // If this item is selected, scroll to it
                        if is_selected {
                            ui.scroll_to_rect(egui::Rect::from_min_size(
                                ui.cursor().min,
                                egui::Vec2::new(ui.available_width(), 30.0)
                            ), Some(egui::Align::Center));
                        }
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
                                            self.theme.done_color
                                        } else {
                                            self.theme.foreground
                                        };
                                        
                                        ui.label(egui::RichText::new(checkbox_text)
                                            .color(if done { self.theme.accent } else { self.theme.border })
                                            .monospace());
                                        
                                        // Show project name with project-specific color if present
                                        if let Some(ref proj) = project {
                                            let project_color = self.get_project_color(proj);
                                            ui.label(egui::RichText::new(format!("{}: ", proj))
                                                .color(project_color)
                                                .strong());
                                        }
                                        
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Enforce minimum window size at runtime
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                let current_size = rect.size();
                if current_size.x < 480.0 || current_size.y < 300.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(
                        current_size.x.max(480.0),
                        current_size.y.max(300.0),
                    )));
                }
            }
        });
        
        // Hot-reload theme less frequently to avoid blocking
        if self.last_theme_check.elapsed() > Duration::from_millis(500) {
            let old_bg = self.theme.background;
            self.load_theme();
            let new_bg = self.theme.background;
            
            // Force repaint if theme actually changed
            if old_bg != new_bg {
                ctx.request_repaint();
            }
            
            self.last_theme_check = Instant::now();
        }
        
        // Less frequent repaints to avoid blocking
        ctx.request_repaint_after(Duration::from_millis(500));
        
        self.handle_keyboard(ctx);
        
        // Render project palette if open
        self.render_project_palette(ctx);
        
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
        
        // Apply font configuration from theme
        if let Some(ref font_family) = self.theme.font_family {
            let mut fonts = egui::FontDefinitions::default();
            
            // Try to load the user's font family
            if let Ok(font_data) = std::fs::read(format!("/usr/share/fonts/TTF/{}.ttf", font_family))
                .or_else(|_| std::fs::read(format!("/usr/share/fonts/truetype/{}/{}.ttf", font_family.to_lowercase(), font_family)))
                .or_else(|_| std::fs::read(format!("/System/Library/Fonts/{}.ttf", font_family)))
                .or_else(|_| std::fs::read(format!("/System/Library/Fonts/{}.otf", font_family)))
            {
                fonts.font_data.insert(
                    font_family.clone(),
                    egui::FontData::from_owned(font_data),
                );
                fonts.families.entry(egui::FontFamily::Proportional).or_default()
                    .insert(0, font_family.clone());
                fonts.families.entry(egui::FontFamily::Monospace).or_default()
                    .insert(0, font_family.clone());
                ctx.set_fonts(fonts);
            }
        }
        
        // Apply font size (use effective font size that considers user override)
        let font_size = self.get_effective_font_size();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(font_size, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(font_size, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(font_size * 0.8, egui::FontFamily::Proportional),
        );
        
        ctx.set_style(style);
        
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_color).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    
                    // Header with ASCII art and filters aligned at bottom
                    ui.horizontal(|ui| {
                        // ASCII art on the left
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                            let ascii_art = vec![
                                "  ██████  ███    ███  █████  ██████   ██████ ",
                                " ██    ██ ████  ████ ██   ██ ██   ██ ██    ██",
                                " ██    ██ ██ ████ ██ ███████ ██   ██ ██    ██",
                                " ██    ██ ██  ██  ██ ██   ██ ██   ██ ██    ██",
                                "  ██████  ██      ██ ██   ██ ██████   ██████ ",
                            ];
                            
                            ui.vertical(|ui| {
                                for line in ascii_art {
                                    ui.label(egui::RichText::new(line)
                                        .color(self.theme.accent)
                                        .size(8.0)
                                        .monospace());
                                }
                            });
                        });
                        
                        // Filter info on the right, bottom-aligned
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                            let project_name = match &self.project_filter {
                                ProjectFilter::All => "All",
                                ProjectFilter::NoProject => "No project",
                                ProjectFilter::Project(name) => name,
                            };
                            
                            ui.label(egui::RichText::new(format!("Project: {}", project_name))
                                .color(self.theme.done_color)
                                .size(12.0));
                            ui.label(egui::RichText::new(" | ")
                                .color(self.theme.border)
                                .size(12.0));
                            ui.label(egui::RichText::new(format!("Filter: {}", self.filter.name()))
                                .color(self.theme.done_color)
                                .size(12.0));
                        });
                    });
                    
                    ui.separator();
                    
                    // Conditional Search bar
                    if self.show_search {
                        ui.horizontal(|ui| {
                            ui.label("🔍");
                            let search_response = ui.add(
                                egui::TextEdit::singleline(&mut self.search)
                                    .hint_text("Type to search... (Esc to close)")
                                    .desired_width(ui.available_width() - 20.0)
                            );
                            search_response.request_focus();
                        });
                        ui.separator();
                    }
                    
                    // Todo list
                    self.render_todo_list(ui);
                    
                    ui.separator();
                    
                    // Help text - make it more visible for debugging
                    ui.separator();
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        let help_text = if self.editing.is_some() {
                            "Enter: Save | Esc: Cancel"
                        } else {
                            "j/k: Move | Enter: Edit | a: Add | x: Toggle | dd: Delete | f: Filter | p: Project | Shift+S: Search | Shift+P: Projects | c: Clear Filters | Ctrl+/- : Font Size"
                        };
                        
                        let help_size = self.get_effective_font_size() * 0.9;
                        // Make it more prominent temporarily
                        ui.label(egui::RichText::new("HELP:")
                            .color(egui::Color32::WHITE)
                            .size(help_size)
                            .strong());
                        ui.label(egui::RichText::new(help_text)
                            .color(self.theme.foreground)
                            .size(help_size));
                    });
                    ui.add_space(5.0);
                    
                    if self.delete_mode {
                        ui.horizontal(|ui| {
                            let delete_size = self.get_effective_font_size() * 0.9;
                            ui.label(egui::RichText::new("Press 'd' again to delete selected item")
                                .color(egui::Color32::from_rgb(255, 100, 100))
                                .size(delete_size));
                        });
                    }
                });
            });
    }
}

fn handle_cli_command(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 2 {
        return Ok(()); // No CLI args, run GUI
    }
    
    match args[1].as_str() {
        "add" => {
            if args.len() < 3 {
                eprintln!("Usage: omado add \"<task>\"");
                std::process::exit(1);
            }
            
            let task_text = args[2].clone();
            let (text, project) = TodoApp::parse_todo_text(&task_text);
            
            let todo = Todo {
                text,
                project,
                done: false,
            };
            
            // Load existing todos
            let storage_path = TodoApp::get_storage_path();
            let mut todos = Vec::new();
            
            if let Ok(content) = fs::read_to_string(&storage_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("[ ] ") {
                        let (text, project) = TodoApp::parse_todo_text(&line[4..]);
                        todos.push(Todo {
                            text,
                            done: false,
                            project,
                        });
                    } else if line.starts_with("[x] ") {
                        let (text, project) = TodoApp::parse_todo_text(&line[4..]);
                        todos.push(Todo {
                            text,
                            done: true,
                            project,
                        });
                    }
                }
            }
            
            // Add new todo
            todos.push(todo.clone());
            
            // Save todos
            let mut content = String::new();
            for todo in &todos {
                let prefix = if todo.done { "[x]" } else { "[ ]" };
                let display_text = if let Some(ref project) = todo.project {
                    format!("{}: {}", project, todo.text)
                } else {
                    todo.text.clone()
                };
                content.push_str(&format!("{} {}\n", prefix, display_text));
            }
            fs::write(&storage_path, content)?;
            
            // Confirmation message
            if let Some(ref project) = todo.project {
                println!("✓ Added task to project '{}': {}", project, todo.text);
            } else {
                println!("✓ Added task: {}", todo.text);
            }
            
            std::process::exit(0);
        }
        "help" | "--help" | "-h" => {
            println!("omado - Simple todo management");
            println!();
            println!("USAGE:");
            println!("    omado                    Launch GUI");
            println!("    omado add \"<task>\"       Add a new task");
            println!("    omado help               Show this help");
            println!();
            println!("EXAMPLES:");
            println!("    omado add \"Buy groceries\"");
            println!("    omado add \"work: Fix parser bug\"");
            println!("    omado add \"personal: Call mom\"");
            
            std::process::exit(0);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Run 'omado help' for usage information.");
            std::process::exit(1);
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    
    // Handle CLI commands
    if let Err(e) = handle_cli_command(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    
    // Launch GUI if no CLI command was handled
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([480.0, 300.0]) // Minimum width to fit ASCII art + padding
            .with_decorations(false)
            .with_resizable(true)
            .with_title("omado")
            .with_app_id("omado"),
        ..Default::default()
    };
    
    eframe::run_native(
        "omado",
        options,
        Box::new(|_cc| Ok(Box::new(TodoApp::new()))),
    )
}