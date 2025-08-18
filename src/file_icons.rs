use std::path::Path;

/// Get the appropriate emoji icon for a file based on its extension or name
pub fn get_file_icon(path: &Path) -> &'static str {
    if path.is_dir() {
        return "📁";
    }
    
    // Get file extension
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // Get file name for special cases
    let file_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // Check for special file names first
    match file_name.as_str() {
        "readme.md" | "readme.txt" | "readme" => "📖",
        "license" | "license.txt" | "license.md" => "📄",
        "dockerfile" => "🐳",
        "makefile" => "🔨",
        "cargo.toml" | "cargo.lock" => "📦",
        "package.json" | "package-lock.json" => "📦",
        "yarn.lock" => "🧶",
        "gemfile" | "gemfile.lock" => "💎",
        "pipfile" | "pipfile.lock" => "🐍",
        "requirements.txt" => "🐍",
        "composer.json" | "composer.lock" => "🎼",
        ".gitignore" | ".gitattributes" => "🙈",
        ".env" | ".env.local" | ".env.example" => "⚙️",
        _ => {
            // Check by file extension
            match extension.as_str() {
                // Programming languages
                "rs" => "🦀",
                "js" | "mjs" => "💛",
                "ts" => "🔷",
                "jsx" | "tsx" => "⚛️",
                "py" => "🐍",
                "go" => "🐹",
                "java" => "☕",
                "kt" | "kts" => "🎯",
                "swift" => "🐦",
                "cpp" | "cc" | "cxx" | "c++" => "⚡",
                "c" => "🔧",
                "h" | "hpp" => "📋",
                "cs" => "🔷",
                "php" => "🐘",
                "rb" => "💎",
                "lua" => "🌙",
                "r" => "📊",
                "dart" => "🎯",
                "scala" => "🔺",
                "clj" | "cljs" => "🤖",
                "hs" => "λ",
                "elm" => "🌳",
                "ex" | "exs" => "💧",
                "erl" => "☎️",
                "ml" | "mli" => "🐪",
                "fs" | "fsi" | "fsx" => "📘",
                "nim" => "👑",
                "cr" => "💎",
                "zig" => "⚡",
                
                // Web technologies
                "html" | "htm" => "🌐",
                "css" => "🎨",
                "scss" | "sass" => "💅",
                "less" => "📘",
                "vue" => "💚",
                "svelte" => "🧡",
                "angular" => "🅰️",
                
                // Data formats
                "json" => "📊",
                "xml" => "📄",
                "yaml" | "yml" => "📄",
                "toml" => "📄",
                "ini" | "cfg" | "conf" => "⚙️",
                "csv" => "📊",
                "sql" => "🗃️",
                
                // Documentation
                "md" | "markdown" => "📝",
                "txt" => "📄",
                "rtf" => "📄",
                "pdf" => "📕",
                "doc" | "docx" => "📘",
                "xls" | "xlsx" => "📗",
                "ppt" | "pptx" => "📙",
                
                // Images
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tiff" => "🖼️",
                "svg" => "🎨",
                "ico" => "🖼️",
                "webp" => "🖼️",
                
                // Audio/Video
                "mp3" | "wav" | "flac" | "aac" => "🎵",
                "mp4" | "avi" | "mkv" | "mov" | "wmv" => "🎬",
                
                // Archives
                "zip" | "rar" | "7z" | "tar" | "gz" | "xz" | "bz2" => "📦",
                
                // Scripts
                "sh" | "bash" | "zsh" | "fish" => "📜",
                "bat" | "cmd" => "📜",
                "ps1" => "📜",
                
                // Other
                "log" => "📋",
                "lock" => "🔒",
                "key" | "pem" | "crt" | "cert" => "🔑",
                "tmp" | "temp" => "🗑️",
                "bak" | "backup" => "💾",
                
                // Default for unknown files
                _ => "📄",
            }
        }
    }
}

/// Get directory icon (can be used for expanded/collapsed states)
pub fn get_directory_icon(is_expanded: bool) -> &'static str {
    if is_expanded {
        "📂"
    } else {
        "📁"
    }
}

/// Get a simple file type indicator (non-emoji version for contexts that don't support emoji)
#[allow(dead_code)]
pub fn get_file_type_indicator(path: &Path) -> &'static str {
    if path.is_dir() {
        return "D";
    }
    
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match extension.as_str() {
        "rs" => "R",
        "js" | "mjs" => "J",
        "ts" => "T",
        "py" => "P",
        "go" => "G",
        "java" => "J",
        "html" | "htm" => "H",
        "css" => "C",
        "md" | "markdown" => "M",
        "json" => "N",
        "xml" => "X",
        "txt" => "T",
        _ => "F",
    }
}