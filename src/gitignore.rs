use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct GitIgnore {
    patterns: Vec<GitIgnorePattern>,
    repo_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
struct GitIgnorePattern {
    pattern: String,
    is_negation: bool,
    is_directory_only: bool,
    is_absolute: bool,
}

impl GitIgnore {
    pub fn new(repo_root: PathBuf) -> Self {
        let mut gitignore = Self {
            patterns: Vec::new(),
            repo_root,
        };
        gitignore.load_gitignore();
        gitignore
    }

    fn load_gitignore(&mut self) {
        let gitignore_path = self.repo_root.join(".gitignore");
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            for line in content.lines() {
                if let Some(pattern) = self.parse_line(line) {
                    self.patterns.push(pattern);
                }
            }
        }

        // Add common default patterns
        self.add_default_patterns();
    }

    fn parse_line(&self, line: &str) -> Option<GitIgnorePattern> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let mut pattern = line.to_string();
        let mut is_negation = false;
        let mut is_directory_only = false;
        let mut is_absolute = false;

        // Handle negation
        if pattern.starts_with('!') {
            is_negation = true;
            pattern = pattern[1..].to_string();
        }

        // Handle directory-only patterns
        if pattern.ends_with('/') {
            is_directory_only = true;
            pattern.pop();
        }

        // Handle absolute patterns
        if pattern.starts_with('/') {
            is_absolute = true;
            pattern = pattern[1..].to_string();
        }

        Some(GitIgnorePattern {
            pattern,
            is_negation,
            is_directory_only,
            is_absolute,
        })
    }

    fn add_default_patterns(&mut self) {
        // Add some common patterns that should always be ignored
        let default_patterns = vec![".git", ".DS_Store", "Thumbs.db", "*.swp", "*.swo", "*~"];

        for pattern in default_patterns {
            self.patterns.push(GitIgnorePattern {
                pattern: pattern.to_string(),
                is_negation: false,
                is_directory_only: false,
                is_absolute: false,
            });
        }
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        // Convert to relative path from repo root
        let relative_path = if let Ok(rel) = path.strip_prefix(&self.repo_root) {
            rel
        } else {
            // If path is not under repo root, don't ignore it
            return false;
        };

        let path_str = relative_path.to_string_lossy();
        let is_directory = path.is_dir();

        let mut ignored = false;

        for pattern in &self.patterns {
            if self.matches_pattern(pattern, &path_str, is_directory) {
                ignored = !pattern.is_negation;
            }
        }

        ignored
    }

    fn matches_pattern(&self, pattern: &GitIgnorePattern, path: &str, is_directory: bool) -> bool {
        // If pattern is directory-only and path is not a directory, no match
        if pattern.is_directory_only && !is_directory {
            return false;
        }

        let pattern_str = &pattern.pattern;

        // Handle absolute patterns
        if pattern.is_absolute {
            return self.glob_match(pattern_str, path);
        }

        // For relative patterns, check if any part of the path matches
        let path_parts: Vec<&str> = path.split('/').collect();

        // Try matching against the full path
        if self.glob_match(pattern_str, path) {
            return true;
        }

        // Try matching against just the filename
        if let Some(filename) = path_parts.last() {
            if self.glob_match(pattern_str, filename) {
                return true;
            }
        }

        // Try matching against any suffix of the path
        for i in 0..path_parts.len() {
            let suffix = path_parts[i..].join("/");
            if self.glob_match(pattern_str, &suffix) {
                return true;
            }
        }

        false
    }

    fn glob_match(&self, pattern: &str, text: &str) -> bool {
        // Simple glob matching implementation
        // This is a basic implementation - could be enhanced with a proper glob library

        if pattern == text {
            return true;
        }

        if pattern.contains('*') {
            return self.wildcard_match(pattern, text);
        }

        false
    }

    fn wildcard_match(&self, pattern: &str, text: &str) -> bool {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();

        self.wildcard_match_recursive(&pattern_chars, &text_chars, 0, 0)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn wildcard_match_recursive(
        &self,
        pattern: &[char],
        text: &[char],
        p: usize,
        t: usize,
    ) -> bool {
        if p >= pattern.len() {
            return t >= text.len();
        }

        if pattern[p] == '*' {
            // Try matching zero characters
            if self.wildcard_match_recursive(pattern, text, p + 1, t) {
                return true;
            }
            // Try matching one or more characters
            for i in t..text.len() {
                if self.wildcard_match_recursive(pattern, text, p + 1, i + 1) {
                    return true;
                }
            }
            false
        } else if t >= text.len() {
            false
        } else if pattern[p] == '?' || pattern[p] == text[t] {
            self.wildcard_match_recursive(pattern, text, p + 1, t + 1)
        } else {
            false
        }
    }
}
