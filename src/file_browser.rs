use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_drive: bool,
}

pub struct FileBrowser {
    pub current_path: PathBuf,
    pub entries: Vec<DirEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

impl FileBrowser {
    pub fn new() -> Self {
        let current_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut browser = FileBrowser {
            current_path: current_path.clone(),
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        };
        browser.refresh();
        browser
    }

    fn is_at_drive_root(&self) -> bool {
        // Check if we're at a drive root like "C:\" or "D:\"
        let path_str = self.current_path.to_string_lossy();
        let path_str = path_str.trim_end_matches('\\').trim_end_matches('/');
        path_str.len() == 2 && path_str.ends_with(':')
    }

    fn get_available_drives() -> Vec<DirEntry> {
        let mut drives = Vec::new();
        // Check drives A-Z
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive_path);
            if path.exists() {
                drives.push(DirEntry {
                    name: format!("{}: Drive", letter as char),
                    path,
                    is_dir: true,
                    is_drive: true,
                });
            }
        }
        drives
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;

        // If at drive root, show available drives
        if self.is_at_drive_root() {
            self.entries.extend(Self::get_available_drives());
            
            // Add separator
            self.entries.push(DirEntry {
                name: "─────────────────".to_string(),
                path: self.current_path.clone(),
                is_dir: false,
                is_drive: false,
            });
        }

        // Add parent directory option if not at root
        if self.current_path.parent().is_some() {
            self.entries.push(DirEntry {
                name: "..".to_string(),
                path: self.current_path.parent().unwrap().to_path_buf(),
                is_dir: true,
                is_drive: false,
            });
        }

        // Read directory entries
        if let Ok(read_dir) = fs::read_dir(&self.current_path) {
            let mut dirs: Vec<DirEntry> = read_dir
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        Some(DirEntry {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: path.clone(),
                            is_dir: true,
                            is_drive: false,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Sort directories alphabetically
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            self.entries.extend(dirs);
        }

        // Ensure selected index is valid
        if self.selected_index >= self.entries.len() && !self.entries.is_empty() {
            self.selected_index = self.entries.len() - 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            // Scroll up if selection goes above visible area
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    pub fn move_down(&mut self, visible_height: usize) {
        if self.selected_index < self.entries.len().saturating_sub(1) {
            self.selected_index += 1;
            // Scroll down if selection goes below visible area
            if self.selected_index >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected_index - visible_height + 1;
            }
        }
    }

    pub fn enter_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_index) {
            if entry.is_dir {
                self.current_path = entry.path.clone();
                self.refresh();
            }
        }
    }

    pub fn get_selected_path(&self) -> PathBuf {
        self.current_path.clone()
    }

    pub fn get_current_path_string(&self) -> String {
        self.current_path.to_string_lossy().to_string()
    }
}
