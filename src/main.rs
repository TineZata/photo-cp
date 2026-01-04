use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{
    io,
    path::Path,
};
use walkdir::WalkDir;

mod photo_organizer;
use photo_organizer::PhotoOrganizer;

mod file_browser;
use file_browser::FileBrowser;

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputMode {
    BrowsingSource,
    BrowsingDest,
    ReadyToProcess,
    Processing,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ProcessingState {
    NotStarted,
    InProgress,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HierarchyMode {
    None,
    YearMonth,
}

struct App {
    source_folder: String,
    dest_folder: String,
    input_mode: InputMode,
    hierarchy_mode: HierarchyMode,
    source_browser: FileBrowser,
    dest_browser: FileBrowser,
    progress: f64,
    total_files: usize,
    processed_files: usize,
    current_file: String,
    status_message: String,
    errors: Vec<String>,
    processing_state: ProcessingState,
    photo_files: Vec<walkdir::DirEntry>,
    current_index: usize,
}

impl App {
    fn new() -> App {
        App {
            source_folder: String::new(),
            dest_folder: String::new(),
            input_mode: InputMode::BrowsingSource,
            hierarchy_mode: HierarchyMode::YearMonth,
            source_browser: FileBrowser::new(),
            dest_browser: FileBrowser::new(),
            progress: 0.0,
            total_files: 0,
            processed_files: 0,
            current_file: String::new(),
            status_message: String::from("Browse to select source folder"),
            errors: Vec::new(),
            processing_state: ProcessingState::NotStarted,
            photo_files: Vec::new(),
            current_index: 0,
        }
    }

    fn toggle_hierarchy(&mut self) {
        self.hierarchy_mode = match self.hierarchy_mode {
            HierarchyMode::None => HierarchyMode::YearMonth,
            HierarchyMode::YearMonth => HierarchyMode::None,
        };
    }

    fn confirm_source(&mut self) {
        self.source_folder = self.source_browser.get_current_path_string();
        self.input_mode = InputMode::BrowsingDest;
        self.status_message = String::from("Browse to select destination folder");
    }

    fn confirm_dest(&mut self) {
        self.dest_folder = self.dest_browser.get_current_path_string();
        self.input_mode = InputMode::ReadyToProcess;
        self.status_message = String::from("Press 's' to start copying");
    }

    fn start_processing(&mut self) -> io::Result<()> {
        if self.source_folder.is_empty() || self.dest_folder.is_empty() {
            self.status_message = String::from("Error: Both folders must be specified");
            return Ok(());
        }

        if !Path::new(&self.source_folder).exists() {
            self.status_message = String::from("Error: Source folder does not exist");
            return Ok(());
        }

        self.input_mode = InputMode::Processing;
        self.status_message = String::from("Scanning files...");
        self.progress = 0.0;
        self.processed_files = 0;
        self.errors.clear();

        // Count total photo files
        self.total_files = WalkDir::new(&self.source_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_photo_file(e.path()))
            .count();

        if self.total_files == 0 {
            self.status_message = String::from("No photo files found in source folder");
            self.input_mode = InputMode::Completed;
            return Ok(());
        }

        self.status_message = format!("Found {} photo files. Processing...", self.total_files);

        Ok(())
    }

    fn init_processing(&mut self) {
        self.processing_state = ProcessingState::InProgress;
        self.photo_files = WalkDir::new(&self.source_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_photo_file(e.path()))
            .collect();
        self.current_index = 0;
    }

    fn process_next_batch(&mut self, batch_size: usize) -> io::Result<bool> {
        if self.processing_state == ProcessingState::Cancelled {
            self.input_mode = InputMode::Completed;
            self.status_message = format!(
                "Cancelled! Processed {} of {} files with {} errors",
                self.processed_files,
                self.total_files,
                self.errors.len()
            );
            return Ok(true); // Done
        }

        let organizer = PhotoOrganizer::new(
            self.source_folder.clone(),
            self.dest_folder.clone(),
            self.hierarchy_mode == HierarchyMode::YearMonth,
        );

        let end_index = std::cmp::min(self.current_index + batch_size, self.photo_files.len());
        
        for i in self.current_index..end_index {
            let entry = &self.photo_files[i];
            self.current_file = entry
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            match organizer.copy_photo(entry.path()) {
                Ok(_) => {}
                Err(e) => {
                    self.errors
                        .push(format!("{}: {}", self.current_file, e));
                }
            }

            self.processed_files += 1;
            self.progress = (self.processed_files as f64 / self.total_files as f64) * 100.0;
        }

        self.current_index = end_index;

        if self.current_index >= self.photo_files.len() {
            // No more files
            self.input_mode = InputMode::Completed;
            self.status_message = format!(
                "Completed! Processed {} files with {} errors",
                self.processed_files,
                self.errors.len()
            );
            return Ok(true); // Done
        }

        Ok(false) // Not done yet
    }
}

fn is_photo_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(
            ext.as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "heic" | "heif" | "raw" | "cr2" | "nef" | "arw"
        )
    } else {
        false
    }
}

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app)).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Process files in batches when in Processing mode
        if app.input_mode == InputMode::Processing {
            // Process a small batch of files
            app.process_next_batch(5)?;
        }

        // Use poll with a timeout to allow processing to continue
        let timeout = if app.input_mode == InputMode::Processing {
            std::time::Duration::from_millis(50)
        } else {
            std::time::Duration::from_millis(100)
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                match app.input_mode {
                    InputMode::BrowsingSource => match key.code {
                        KeyCode::Up => app.source_browser.move_up(),
                        KeyCode::Down => app.source_browser.move_down(20),
                        KeyCode::Enter => app.source_browser.enter_selected(),
                        KeyCode::Char(' ') => app.confirm_source(),
                        KeyCode::Char('h') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                            app.toggle_hierarchy();
                        }
                        KeyCode::Esc => return Ok(()),
                        _ => {}
                    },
                    InputMode::BrowsingDest => match key.code {
                        KeyCode::Up => app.dest_browser.move_up(),
                        KeyCode::Down => app.dest_browser.move_down(20),
                        KeyCode::Enter => app.dest_browser.enter_selected(),
                        KeyCode::Char(' ') => app.confirm_dest(),
                        KeyCode::Char('h') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                            app.toggle_hierarchy();
                        }
                        KeyCode::Backspace => {
                            app.input_mode = InputMode::BrowsingSource;
                            app.source_folder.clear();
                            app.status_message = String::from("Browse to select source folder");
                        }
                        KeyCode::Esc => return Ok(()),
                        _ => {}
                    },
                    InputMode::ReadyToProcess => match key.code {
                        KeyCode::Char('s') => {
                            app.start_processing()?;
                            app.init_processing();
                        }
                        KeyCode::Char('h') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                            app.toggle_hierarchy();
                        }
                        KeyCode::Backspace => {
                            app.input_mode = InputMode::BrowsingDest;
                            app.dest_folder.clear();
                            app.status_message = String::from("Browse to select destination folder");
                        }
                        KeyCode::Esc => return Ok(()),
                        _ => {}
                    },
                    InputMode::Processing => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.processing_state = ProcessingState::Cancelled;
                            app.status_message = String::from("Cancelling... please wait");
                        }
                        _ => {}
                    },
                    InputMode::Completed => {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                            return Ok(());
                        }
                        if key.code == KeyCode::Char('r') {
                            *app = App::new();
                        }
                    }
                }
            }
        }
        } // closing brace for event::poll
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("Photo Organizer & Copy Tool")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Source folder display
    let source_style = if app.input_mode == InputMode::BrowsingSource {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let source_text = if app.source_folder.is_empty() {
        "Source: (browsing...)".to_string()
    } else {
        format!("Source: {}", app.source_folder)
    };
    let source_input = Paragraph::new(source_text)
        .style(source_style)
        .block(Block::default().borders(Borders::ALL).title("Source Folder"));
    f.render_widget(source_input, chunks[1]);

    // Destination folder display
    let dest_style = if app.input_mode == InputMode::BrowsingDest {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let dest_text = if app.dest_folder.is_empty() {
        "Destination: (not set)".to_string()
    } else {
        format!("Destination: {}", app.dest_folder)
    };
    let dest_input = Paragraph::new(dest_text)
        .style(dest_style)
        .block(Block::default().borders(Borders::ALL).title("Destination Folder"));
    f.render_widget(dest_input, chunks[2]);

    // Hierarchy mode
    let hierarchy_text = format!(
        "Organization: {} (Ctrl+H to toggle)",
        match app.hierarchy_mode {
            HierarchyMode::None => "Flat (no hierarchy)",
            HierarchyMode::YearMonth => "Year/Month hierarchy",
        }
    );
    let hierarchy = Paragraph::new(hierarchy_text)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(hierarchy, chunks[3]);

    // Main content area - File browser or progress
    match app.input_mode {
        InputMode::BrowsingSource => {
            render_file_browser(f, chunks[4], &app.source_browser, "Select Source Folder");
        }
        InputMode::BrowsingDest => {
            render_file_browser(f, chunks[4], &app.dest_browser, "Select Destination Folder");
        }
        InputMode::ReadyToProcess => {
            let help_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Ready to copy photos!",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!("  Source: {}", app.source_folder)),
                Line::from(format!("  Destination: {}", app.dest_folder)),
                Line::from(format!(
                    "  Organization: {}",
                    match app.hierarchy_mode {
                        HierarchyMode::None => "Flat",
                        HierarchyMode::YearMonth => "Year/Month",
                    }
                )),
                Line::from(""),
                Line::from("Press 's' to start copying"),
                Line::from("Press Backspace to change destination"),
                Line::from("Press ESC to quit"),
            ];
            let help = Paragraph::new(help_text)
                .style(Style::default())
                .block(Block::default().borders(Borders::ALL).title("Ready"));
            f.render_widget(help, chunks[4]);
        }
        InputMode::Processing | InputMode::Completed => {
            let progress_text = if app.input_mode == InputMode::Processing {
                format!(
                    "{}/{} - {} (Press ESC or 'q' to cancel)",
                    app.processed_files, app.total_files, app.current_file
                )
            } else {
                format!(
                    "{}/{} - {}",
                    app.processed_files, app.total_files, app.current_file
                )
            };

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::Black)
                        .add_modifier(Modifier::ITALIC),
                )
                .percent(app.progress as u16)
                .label(progress_text);
            f.render_widget(gauge, chunks[4]);
        }
    }

    // Status message
    let status_color = if app.status_message.contains("Error") {
        Color::Red
    } else if app.input_mode == InputMode::Completed {
        Color::Green
    } else {
        Color::White
    };

    let mut status_lines = vec![Line::from(Span::styled(
        &app.status_message,
        Style::default().fg(status_color),
    ))];

    if app.input_mode == InputMode::Completed && !app.errors.is_empty() {
        status_lines.push(Line::from(""));
        status_lines.push(Line::from(Span::styled(
            format!("Errors ({}): Press 'r' to restart, 'q' to quit", app.errors.len()),
            Style::default().fg(Color::Red),
        )));
        for (i, error) in app.errors.iter().take(3).enumerate() {
            status_lines.push(Line::from(format!("  {}: {}", i + 1, error)));
        }
        if app.errors.len() > 3 {
            status_lines.push(Line::from(format!("  ... and {} more", app.errors.len() - 3)));
        }
    }

    let status = Paragraph::new(status_lines).block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[5]);
}

fn render_file_browser(f: &mut Frame, area: ratatui::layout::Rect, browser: &FileBrowser, title: &str) {
    let current_path = browser.get_current_path_string();
    
    let items: Vec<ListItem> = browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let (prefix, style_color) = if entry.is_drive {
                ("💾 ", Color::Cyan)
            } else if entry.name == ".." {
                ("📁 ", Color::White)
            } else if entry.name.starts_with("─") {
                ("", Color::DarkGray)
            } else {
                ("📂 ", Color::White)
            };
            
            let content = format!("{}{}", prefix, entry.name);
            let style = if i == browser.selected_index && entry.is_dir {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else if entry.is_drive {
                Style::default().fg(style_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(style_color)
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} - {}", title, current_path))
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(browser.selected_index));
    
    f.render_stateful_widget(list, area, &mut list_state);

    // Add help text at bottom of browser
    let help_line = match title {
        s if s.contains("Source") => "↑↓: Navigate | Enter: Open/Switch | Space: Select this folder | ESC: Quit",
        _ => "↑↓: Navigate | Enter: Open/Switch | Space: Select this folder | Backspace: Go back | ESC: Quit",
    };
    
    let help_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.y + area.height - 2,
        width: area.width.saturating_sub(2),
        height: 1,
    };
    
    let help = Paragraph::new(help_line)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help, help_area);
}
