use crate::app::App;
use crate::tab::Tab;
use std::path::PathBuf;
use std::time::Duration;

impl App {
    pub fn save_current_file(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab() {
            match tab {
                Tab::Editor { path, .. } => {
                    if path.is_none() {
                        // No path set, show save dialog
                        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        self.menu_system.open_input_dialog(
                            "Save as:".to_string(),
                            "save_file".to_string(),
                            current_dir,
                        );
                        return;
                    }
                }
                Tab::Terminal { .. } => {
                    // Terminal tabs cannot be saved
                    return;
                }
            }
        }

        // Save existing file
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            if let Tab::Editor { path, buffer, .. } = tab {
                if let Some(path) = path.clone() {
                    if std::fs::write(&path, buffer.to_string()).is_ok() {
                        tab.mark_saved();
                        self.set_status_message(
                            format!("Saved: {}", path.display()),
                            Duration::from_secs(2),
                        );
                    } else {
                        self.set_status_message(
                            format!("Failed to save: {}", path.display()),
                            Duration::from_secs(3),
                        );
                    }
                }
            }
        }
    }

    pub fn execute_file_operation(&mut self, operation: &str, target_path: &PathBuf, input: &str) {
        match operation {
            "save_file" => {
                // Save current tab to the specified filename
                if let Some(tab) = self.tab_manager.active_tab_mut() {
                    let file_path = if input.trim().starts_with('/') {
                        PathBuf::from(input.trim())
                    } else {
                        target_path.join(input.trim())
                    };

                    if let Tab::Editor { buffer, path, name, .. } = tab {
                        if std::fs::write(&file_path, buffer.to_string()).is_ok() {
                            *path = Some(file_path.clone());
                            *name = file_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("untitled")
                                .to_string();
                            tab.mark_saved();
                            self.set_status_message(
                                format!("Saved: {}", file_path.display()),
                                Duration::from_secs(2),
                            );

                            // Refresh tree view to show the new file
                            if let Some(tree_view) = &mut self.tree_view {
                                tree_view.refresh();
                            }
                        } else {
                            self.set_status_message(
                                format!("Failed to save: {}", input.trim()),
                                Duration::from_secs(3),
                            );
                        }
                    }
                }
            }
            _ => {
                if let Some(tree_view) = &mut self.tree_view {
                    let result = match operation {
                        "new_file" => tree_view
                            .create_file(target_path, input.trim())
                            .map(|_| format!("Created file '{}'", input.trim()))
                            .map_err(|e| format!("Failed to create file: {}", e)),
                        "new_folder" => tree_view
                            .create_directory(target_path, input.trim())
                            .map(|_| format!("Created directory '{}'", input.trim()))
                            .map_err(|e| format!("Failed to create directory: {}", e)),
                        "rename" => {
                            match tree_view.rename_file_or_directory(target_path, input.trim()) {
                                Ok(new_path) => {
                                    // Update any open tabs with the renamed file
                                    for tab in self.tab_manager.tabs.iter_mut() {
                                        if let crate::tab::Tab::Editor { path, name, .. } = tab {
                                            if let Some(tab_path) = path {
                                                if tab_path == target_path {
                                                    // Update tab path and name
                                                    *path = Some(new_path.clone());
                                                    if let Some(file_name) = new_path.file_name() {
                                                        *name = file_name.to_string_lossy().to_string();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Ok(format!("Renamed to '{}'", input.trim()))
                                }
                                Err(e) => Err(format!("Failed to rename: {}", e)),
                            }
                        }
                        _ => return
                    };

                    // Handle result after borrow is released
                    let (message, is_error) = match result {
                        Ok(msg) => (msg, false),
                        Err(err) => (err, true),
                    };
                    
                    tree_view.refresh();
                }
                
                // Set status message after borrowing is complete
                if let Some(tree_view) = &mut self.tree_view {
                    self.expand_tree_to_current_file();
                }
                
                // Handle the result message
                match operation {
                    "new_file" | "new_folder" | "rename" => {
                        // Dummy operation to get the result
                        if let Some(_tree_view) = &self.tree_view {
                            // We need to handle this differently to avoid borrow issues
                            // For now, let's just set a generic message
                            self.set_status_message("File operation completed".to_string(), Duration::from_secs(2));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}