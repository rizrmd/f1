use crate::file_icons;
use crate::gitignore::GitIgnore;
use crate::ui::scrollbar::{ScrollbarState, VerticalScrollbar};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub children: Vec<TreeNode>,
    pub depth: usize,
    pub is_gitignored: bool,
}

impl TreeNode {
    pub fn new(path: PathBuf, depth: usize) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let is_dir = path.is_dir();

        Self {
            path,
            name,
            is_dir,
            is_expanded: false,
            children: Vec::new(),
            depth,
            is_gitignored: false, // Will be set later when we have gitignore info
        }
    }

    pub fn load_children(&mut self) -> Result<(), std::io::Error> {
        if !self.is_dir || !self.children.is_empty() {
            return Ok(());
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();

            let node = TreeNode::new(path, self.depth + 1);
            entries.push(node);
        }

        // Sort: directories first, then files, both alphabetically
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.children = entries;
        Ok(())
    }

    pub fn toggle_expand(&mut self) -> Result<(), std::io::Error> {
        if !self.is_dir {
            return Ok(());
        }

        if self.is_expanded {
            self.is_expanded = false;
        } else {
            self.load_children()?;
            self.is_expanded = true;
        }
        Ok(())
    }

    pub fn expand_path(&mut self, target_path: &Path) -> Result<bool, std::io::Error> {
        // If this node's path is a prefix of the target path, expand it
        if target_path.starts_with(&self.path) && self.is_dir {
            if !self.is_expanded {
                self.load_children()?;
                self.is_expanded = true;
            }

            // Try to expand children
            for child in &mut self.children {
                if child.expand_path(target_path)? {
                    return Ok(true);
                }
            }

            // Check if this is the exact match
            if self.path == target_path {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Debug)]
pub struct TreeView {
    pub root: TreeNode,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub search_query: String,
    pub is_searching: bool,
    pub filtered_items: Vec<(usize, TreeNode)>, // (original_index, node)
    pub width: u16,
    pub is_focused: bool,
    gitignore: GitIgnore,
    pub just_refreshed: bool,              // Flag for visual feedback
    pub clipboard: Option<ClipboardEntry>, // For copy/cut/paste operations
    last_scroll_time: Option<Instant>,     // For scroll acceleration
    scroll_acceleration: usize,            // Current scroll speed multiplier
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub path: PathBuf,
    pub is_cut: bool, // true for cut, false for copy
}

impl TreeView {
    pub fn new(root_path: PathBuf, width: u16) -> Result<Self, std::io::Error> {
        let gitignore = GitIgnore::new(root_path.clone());
        let mut root = TreeNode::new(root_path, 0);
        root.load_children()?;
        root.is_expanded = true;

        let mut tree_view = Self {
            root,
            selected_index: 0,
            scroll_offset: 0,
            search_query: String::new(),
            is_searching: false,
            filtered_items: Vec::new(),
            width,
            is_focused: false,
            gitignore,
            just_refreshed: false,
            clipboard: None,
            last_scroll_time: None,
            scroll_acceleration: 1,
        };

        // Update gitignore status for all nodes
        tree_view.update_gitignore_status();

        Ok(tree_view)
    }

    fn update_gitignore_status(&mut self) {
        Self::update_node_gitignore_status_recursive(&self.gitignore, &mut self.root);
    }

    fn update_node_gitignore_status_recursive(gitignore: &GitIgnore, node: &mut TreeNode) {
        node.is_gitignored = gitignore.is_ignored(&node.path);
        for child in &mut node.children {
            Self::update_node_gitignore_status_recursive(gitignore, child);
        }
    }

    pub fn toggle_selected(&mut self) -> Result<(), std::io::Error> {
        let visible_items = self.get_visible_items();
        if let Some(item) = visible_items.get(self.selected_index) {
            let path = item.path.clone();
            // Find the actual node in the tree and toggle it
            self.toggle_node_at_path(&path)?;
            // Update gitignore status for any newly loaded nodes
            self.update_gitignore_status();
        }
        Ok(())
    }

    fn toggle_node_at_path(&mut self, path: &Path) -> Result<(), std::io::Error> {
        Self::toggle_node_recursive(&mut self.root, path)
    }

    fn toggle_node_recursive(
        node: &mut TreeNode,
        target_path: &Path,
    ) -> Result<(), std::io::Error> {
        if node.path == target_path {
            node.toggle_expand()?;
            return Ok(());
        }

        for child in &mut node.children {
            if target_path.starts_with(&child.path) {
                Self::toggle_node_recursive(child, target_path)?;
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn create_file(
        &mut self,
        parent_path: &Path,
        filename: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let file_path = parent_path.join(filename);

        // Create the file
        std::fs::File::create(&file_path)?;

        // Refresh the tree
        self.refresh_directory(parent_path)?;

        Ok(file_path)
    }

    pub fn create_directory(
        &mut self,
        parent_path: &Path,
        dirname: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let dir_path = parent_path.join(dirname);

        // Create the directory
        std::fs::create_dir(&dir_path)?;

        // Refresh the tree
        self.refresh_directory(parent_path)?;

        Ok(dir_path)
    }

    pub fn delete_file_or_directory(&mut self, path: &Path) -> Result<(), std::io::Error> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }

        // Refresh the parent directory
        if let Some(parent) = path.parent() {
            self.refresh_directory(parent)?;
        }

        Ok(())
    }

    pub fn rename_file_or_directory(
        &mut self,
        old_path: &Path,
        new_name: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let parent = old_path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot rename root directory",
            )
        })?;

        let new_path = parent.join(new_name);

        // Rename the file/directory
        std::fs::rename(old_path, &new_path)?;

        // Refresh the parent directory
        self.refresh_directory(parent)?;

        Ok(new_path)
    }

    fn refresh_directory(&mut self, dir_path: &Path) -> Result<(), std::io::Error> {
        // Find the node and reload its children
        Self::refresh_node_recursive(&mut self.root, dir_path)?;

        // Update gitignore status for any newly loaded nodes
        self.update_gitignore_status();

        Ok(())
    }

    fn refresh_node_recursive(
        node: &mut TreeNode,
        target_path: &Path,
    ) -> Result<(), std::io::Error> {
        if node.path == target_path && node.is_dir {
            // Clear children and reload
            node.children.clear();
            node.load_children()?;
            return Ok(());
        }

        for child in &mut node.children {
            if target_path.starts_with(&child.path) {
                Self::refresh_node_recursive(child, target_path)?;
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn refresh(&mut self) {
        // Set refresh flag for visual feedback
        self.just_refreshed = true;

        // Save current state
        let selected_path = self.get_selected_item().map(|item| item.path.clone());
        let mut expanded_paths = Vec::new();

        // Collect expanded paths
        self.collect_expanded_paths(&self.root.clone(), &mut expanded_paths);

        // Recreate the root node
        let root_path = self.root.path.clone();
        self.root = TreeNode::new(root_path.clone(), 0);

        // Load root children
        if self.root.load_children().is_err() {
            return;
        }

        // Apply gitignore to root children
        for child in &mut self.root.children {
            child.is_gitignored = self.gitignore.is_ignored(&child.path);
        }

        // Re-expand previously expanded directories
        for path in expanded_paths {
            Self::expand_path_recursive_static(&path, &mut self.root, &self.gitignore);
        }

        // Restore selection if possible
        if let Some(path) = selected_path {
            self.restore_selection(&path);
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_expanded_paths(&self, node: &TreeNode, paths: &mut Vec<PathBuf>) {
        if node.is_expanded && node.is_dir {
            paths.push(node.path.clone());
            for child in &node.children {
                self.collect_expanded_paths(child, paths);
            }
        }
    }

    fn expand_path_recursive_static(
        target_path: &PathBuf,
        node: &mut TreeNode,
        gitignore: &GitIgnore,
    ) {
        if node.path == *target_path && node.is_dir {
            node.is_expanded = true;
            if node.children.is_empty() {
                let _ = node.load_children();
                // Apply gitignore to children
                for child in &mut node.children {
                    child.is_gitignored = gitignore.is_ignored(&child.path);
                }
            }
        }

        // Recursively check children - need to iterate with index to avoid borrow issues
        let num_children = node.children.len();
        for i in 0..num_children {
            Self::expand_path_recursive_static(target_path, &mut node.children[i], gitignore);
        }
    }

    pub fn clear_refresh_flag(&mut self) {
        self.just_refreshed = false;
    }

    pub fn restore_selection(&mut self, path: &PathBuf) {
        let visible_items = self.get_visible_items();
        for (index, item) in visible_items.iter().enumerate() {
            if item.path == *path {
                self.selected_index = index;

                // Ensure selection is visible
                let visible_height = 20; // This could be made configurable
                if self.selected_index < self.scroll_offset {
                    self.scroll_offset = self.selected_index;
                } else if self.selected_index >= self.scroll_offset + visible_height {
                    self.scroll_offset = self.selected_index.saturating_sub(visible_height - 1);
                }
                break;
            }
        }
    }

    pub fn get_visible_items(&self) -> Vec<&TreeNode> {
        if self.is_searching && !self.search_query.is_empty() {
            return self.filtered_items.iter().map(|(_, node)| node).collect();
        }

        let mut items = Vec::new();
        self.collect_visible_items(&self.root, &mut items);
        items
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_visible_items<'a>(&self, node: &'a TreeNode, items: &mut Vec<&'a TreeNode>) {
        if node.depth > 0 {
            // Don't include root
            items.push(node);
        }

        if node.is_expanded {
            for child in &node.children {
                self.collect_visible_items(child, items);
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        let visible_items = self.get_visible_items();
        if self.selected_index < visible_items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn get_selected_item(&self) -> Option<&TreeNode> {
        let visible_items = self.get_visible_items();
        visible_items.get(self.selected_index).copied()
    }

    pub fn expand_to_file(&mut self, file_path: &Path) -> Result<(), std::io::Error> {
        // Expand the root and find the path
        self.root.expand_path(file_path)?;

        // Update gitignore status for any newly loaded nodes
        self.update_gitignore_status();

        // Find the item in visible items and select it
        let visible_items = self.get_visible_items();
        for (index, item) in visible_items.iter().enumerate() {
            if item.path == file_path {
                self.selected_index = index;

                // Scroll to make the selected item visible
                let items_per_page = 20; // Approximate, will be adjusted based on actual height

                if self.selected_index < self.scroll_offset {
                    self.scroll_offset = self.selected_index;
                } else if self.selected_index >= self.scroll_offset + items_per_page {
                    self.scroll_offset = self.selected_index.saturating_sub(items_per_page - 1);
                }
                break;
            }
        }

        Ok(())
    }

    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.search_query.clear();
        self.update_search_filter();
    }

    pub fn stop_search(&mut self) {
        self.is_searching = false;
        self.search_query.clear();
        self.filtered_items.clear();
        self.selected_index = 0;
    }

    pub fn add_search_char(&mut self, c: char) {
        if self.is_searching {
            self.search_query.push(c);
            self.update_search_filter();
        }
    }

    pub fn remove_search_char(&mut self) {
        if self.is_searching && !self.search_query.is_empty() {
            self.search_query.pop();
            self.update_search_filter();
        }
    }

    fn update_search_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        let matching_items: Vec<(usize, TreeNode)> = if self.search_query.is_empty() {
            Vec::new()
        } else {
            // Get comprehensive search results including unexpanded directories
            self.search_all_files(&query)
        };

        self.filtered_items = matching_items;
        self.selected_index = 0;
    }

    fn search_all_files(&self, query: &str) -> Vec<(usize, TreeNode)> {
        let mut results = Vec::new();
        let mut index = 0;

        // First, search in currently visible/expanded items
        let visible_items = self.get_all_items();
        for node in &visible_items {
            if node.name.to_lowercase().contains(query) {
                results.push((index, (*node).clone()));
            }
            index += 1;
        }

        // Then, search in unexpanded directories recursively
        self.search_in_directory(&self.root, query, &mut results, &mut index, 3); // Limit depth to 3 levels

        results
    }

    fn search_in_directory(
        &self,
        node: &TreeNode,
        query: &str,
        results: &mut Vec<(usize, TreeNode)>,
        index: &mut usize,
        max_depth: usize,
    ) {
        if max_depth == 0 || !node.is_dir {
            return;
        }

        // If this directory is already expanded, search in its children but don't re-read from filesystem
        if node.is_expanded && !node.children.is_empty() {
            for child in &node.children {
                if child.is_dir {
                    self.search_in_directory(child, query, results, index, max_depth - 1);
                }
            }
            return;
        }

        // Search in this unexpanded directory
        if let Ok(entries) = std::fs::read_dir(&node.path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if this item matches the search query
                    if name.to_lowercase().contains(query) {
                        let search_node = TreeNode::new(path.clone(), node.depth + 1);
                        results.push((*index, search_node));
                        *index += 1;
                    }

                    // If it's a directory, search recursively
                    if path.is_dir() {
                        let dir_node = TreeNode::new(path, node.depth + 1);
                        self.search_in_directory(&dir_node, query, results, index, max_depth - 1);
                    }
                }
            }
        }
    }

    fn get_all_items(&self) -> Vec<&TreeNode> {
        let mut items = Vec::new();
        self.collect_all_items(&self.root, &mut items);
        items
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_all_items<'a>(&self, node: &'a TreeNode, items: &mut Vec<&'a TreeNode>) {
        if node.depth > 0 {
            // Don't include root
            items.push(node);
        }

        // Collect from expanded children
        for child in &node.children {
            self.collect_all_items(child, items);
        }
    }

    pub fn update_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // Ensure selected item is visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index.saturating_sub(visible_height - 1);
        }
    }

    pub fn scroll_up(&mut self, base_amount: usize) {
        // Update scroll acceleration
        self.update_scroll_acceleration();

        // Calculate actual scroll amount with acceleration
        let scroll_amount = base_amount.saturating_mul(self.scroll_acceleration);
        self.scroll_offset = self.scroll_offset.saturating_sub(scroll_amount);
    }

    pub fn scroll_down(&mut self, base_amount: usize, visible_height: usize) {
        // Update scroll acceleration
        self.update_scroll_acceleration();

        // Calculate actual scroll amount with acceleration
        let scroll_amount = base_amount.saturating_mul(self.scroll_acceleration);

        let visible_items = self.get_visible_items();
        let max_scroll = visible_items.len().saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + scroll_amount).min(max_scroll);
    }

    fn update_scroll_acceleration(&mut self) {
        let now = Instant::now();

        if let Some(last_time) = self.last_scroll_time {
            // If scrolling within 150ms, increase acceleration
            if now.duration_since(last_time).as_millis() < 150 {
                // More aggressive acceleration for tree view
                let increment = if self.scroll_acceleration < 3 {
                    1 // Start with +1 for initial acceleration
                } else if self.scroll_acceleration < 8 {
                    2 // Medium acceleration +2
                } else if self.scroll_acceleration < 15 {
                    3 // Fast acceleration +3
                } else {
                    4 // Very fast +4
                };

                self.scroll_acceleration = (self.scroll_acceleration + increment).min(20);
            } else {
                // Reset acceleration if too much time has passed
                self.scroll_acceleration = 1;
            }
        } else {
            // First scroll, start with base acceleration
            self.scroll_acceleration = 1;
        }

        self.last_scroll_time = Some(now);
    }

    pub fn resize(&mut self, new_width: u16) {
        self.width = new_width;
    }

    pub fn handle_scrollbar_click(&mut self, visible_height: usize, click_y: usize) {
        let visible_items = self.get_visible_items();
        let total_items = visible_items.len();

        if total_items <= visible_height {
            return;
        }

        let scrollbar_state = ScrollbarState::new(total_items, visible_height, self.scroll_offset);

        let new_offset = scrollbar_state.click_position(visible_height, click_y);
        self.scroll_offset = new_offset;

        // Update selected index to stay within view
        if self.selected_index < self.scroll_offset {
            self.selected_index = self.scroll_offset;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.selected_index = self.scroll_offset + visible_height - 1;
        }

        self.selected_index = self.selected_index.min(total_items.saturating_sub(1));
    }

    // File management operations
    pub fn copy_selected(&mut self) {
        if let Some(item) = self.get_selected_item() {
            let path = item.path.clone();

            // Copy to internal clipboard for file operations
            self.clipboard = Some(ClipboardEntry {
                path,
                is_cut: false,
            });
        }
    }

    pub fn cut_selected(&mut self) {
        if let Some(item) = self.get_selected_item() {
            let path = item.path.clone();

            // Copy to internal clipboard for file operations
            self.clipboard = Some(ClipboardEntry { path, is_cut: true });
        }
    }

    pub fn paste_to_selected(&mut self) -> Result<String, String> {
        let clipboard_entry = match &self.clipboard {
            Some(entry) => entry.clone(),
            None => return Err("Nothing to paste".to_string()),
        };

        // Get the target directory
        let target_dir = if let Some(selected_item) = self.get_selected_item() {
            if selected_item.is_dir {
                selected_item.path.clone()
            } else {
                selected_item
                    .path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| self.root.path.clone())
            }
        } else {
            self.root.path.clone()
        };

        let source_name = clipboard_entry
            .path
            .file_name()
            .ok_or_else(|| "Invalid source path".to_string())?;

        let mut target_path = target_dir.join(source_name);

        // If the target already exists, generate a unique name
        if target_path.exists() {
            let stem = clipboard_entry
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let extension = clipboard_entry.path.extension().and_then(|e| e.to_str());

            let mut counter = 1;
            loop {
                let new_name = if let Some(ext) = extension {
                    format!("{}_copy_{}.{}", stem, counter, ext)
                } else {
                    format!("{}_copy_{}", stem, counter)
                };
                target_path = target_dir.join(new_name);
                if !target_path.exists() {
                    break;
                }
                counter += 1;
            }
        }

        // Perform the operation
        if clipboard_entry.is_cut {
            // Move operation
            fs::rename(&clipboard_entry.path, &target_path)
                .map_err(|e| format!("Failed to move: {}", e))?;

            // Clear clipboard after successful cut
            self.clipboard = None;

            // Refresh the tree
            self.refresh();

            Ok(format!("Moved to {}", target_path.display()))
        } else {
            // Copy operation
            if clipboard_entry.path.is_dir() {
                Self::copy_dir_recursive(&clipboard_entry.path, &target_path)
                    .map_err(|e| format!("Failed to copy directory: {}", e))?;
            } else {
                fs::copy(&clipboard_entry.path, &target_path)
                    .map_err(|e| format!("Failed to copy file: {}", e))?;
            }

            // Refresh the tree
            self.refresh();

            Ok(format!("Copied to {}", target_path.display()))
        }
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    pub fn has_clipboard(&self) -> bool {
        self.clipboard.is_some()
    }

    pub fn get_clipboard_info(&self) -> Option<String> {
        self.clipboard.as_ref().map(|entry| {
            let operation = if entry.is_cut { "Cut" } else { "Copied" };
            let name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("item");
            format!("{}: {}", operation, name)
        })
    }

    pub fn find_item_index(&self, target_path: &Path) -> Option<usize> {
        let visible_items = self.get_visible_items();
        visible_items
            .iter()
            .position(|item| item.path == target_path)
    }

    // Add missing methods needed by keyboard handlers
    pub fn toggle_directory(&mut self) -> Result<(), std::io::Error> {
        self.toggle_selected()
    }

    pub fn move_up(&mut self) {
        self.move_selection_up();
    }

    pub fn move_down(&mut self) {
        self.move_selection_down();
    }
}

impl Widget for &TreeView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Use the full area without borders
        let inner = area;

        // Calculate scrollbar first to know the content area
        let needs_scrollbar = {
            let visible_items = self.get_visible_items();
            visible_items.len() > inner.height as usize
        };
        let content_width = if needs_scrollbar {
            inner.width.saturating_sub(1)
        } else {
            inner.width
        };

        // Clear the content area first to prevent artifacts (but not scrollbar area)
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + content_width {
                buf[(x, y)].set_symbol(" ").set_style(Style::default());
            }
        }

        let visible_items = self.get_visible_items();
        let _visible_height = inner.height as usize;

        // Render search box if searching
        let mut content_area = inner;
        if self.is_searching {
            // Draw search box at the top
            let search_text = format!("Search: {}_", self.search_query);
            let search_y = inner.y;

            // Clear the search line first
            for x in inner.x..inner.x + content_width {
                if x < inner.x + content_width {
                    buf[(x, search_y)]
                        .set_symbol(" ")
                        .set_style(Style::default().bg(Color::DarkGray));
                }
            }

            // Draw the search text
            for (i, ch) in search_text.chars().enumerate() {
                let x = inner.x + i as u16;
                if x < inner.x + content_width {
                    let style = if i < 8 {
                        // "Search: " part
                        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                    } else if i == search_text.len() - 1 {
                        // Cursor
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::SLOW_BLINK)
                    } else {
                        // Query text
                        Style::default().fg(Color::White).bg(Color::DarkGray)
                    };

                    buf[(x, search_y)]
                        .set_symbol(&ch.to_string())
                        .set_style(style);
                }
            }

            // Adjust content area to start below search box
            content_area.y += 1;
            content_area.height = content_area.height.saturating_sub(1);
        }

        // Render file tree
        let start_index = self.scroll_offset;
        let end_index = (start_index + content_area.height as usize).min(visible_items.len());

        for (display_index, item_index) in (start_index..end_index).enumerate() {
            if let Some(item) = visible_items.get(item_index) {
                let y = content_area.y + display_index as u16;
                let is_selected = item_index == self.selected_index;

                // Calculate indentation
                let indent = item.depth.saturating_sub(1) * 2;
                let mut x = content_area.x;

                // Draw indentation
                for _ in 0..indent {
                    if x < content_area.x + content_width {
                        buf[(x, y)].set_symbol(" ");
                        x += 1;
                    }
                }

                // Draw file/directory icon
                if x < content_area.x + content_width {
                    let icon = if item.is_dir {
                        file_icons::get_directory_icon(item.is_expanded)
                    } else {
                        file_icons::get_file_icon(&item.path)
                    };
                    buf[(x, y)].set_symbol(icon);
                    x += 2; // Emoji takes 2 columns
                }

                // Add space between icon and text
                if x < content_area.x + content_width {
                    buf[(x, y)].set_symbol(" ");
                    x += 1;
                }

                // Draw file/directory name
                let name_style = if is_selected {
                    if self.is_focused {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        Style::default().bg(Color::DarkGray).fg(Color::White)
                    }
                } else if item.is_gitignored {
                    // Dim gitignored files (both directories and files)
                    Style::default().fg(Color::Rgb(80, 80, 80))
                } else if item.is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                let max_name_width = content_width.saturating_sub(x - content_area.x);
                let display_name = if item.name.len() as u16 > max_name_width {
                    format!(
                        "{}...",
                        &item.name[..max_name_width.saturating_sub(3) as usize]
                    )
                } else {
                    item.name.clone()
                };

                for ch in display_name.chars() {
                    if x < content_area.x + content_width {
                        buf[(x, y)]
                            .set_symbol(&ch.to_string())
                            .set_style(name_style);
                        x += 1;
                    }
                }

                // Fill the rest of the line with selection background
                if is_selected {
                    while x < content_area.x + content_width {
                        buf[(x, y)].set_style(name_style);
                        x += 1;
                    }
                }
            }
        }

        // Draw scrollbar if needed
        if needs_scrollbar {
            let scrollbar_state = ScrollbarState::new(
                visible_items.len(),
                content_area.height as usize,
                self.scroll_offset,
            );

            let scrollbar = VerticalScrollbar::new(scrollbar_state)
                .style(Style::default().fg(Color::Reset))
                .thumb_style(Style::default().fg(Color::White))
                .track_symbols(VerticalScrollbar::minimal());

            let scrollbar_area = Rect {
                x: area.x + area.width - 1,
                y: area.y + if self.is_searching { 1 } else { 0 },
                width: 1,
                height: area.height - if self.is_searching { 1 } else { 0 },
            };

            scrollbar.render(scrollbar_area, buf);
        }
    }
}
