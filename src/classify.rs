#[derive(Debug, Clone, Copy)]
pub enum Category {
    Video,
    Audio,
    Pictures,
    Documents,
    Archives,
    Executables,
    Code,
    Uncategorized,
    Mismatch,
}

impl Category {
    pub fn dir_name(&self) -> &'static str {
        match self {
            Category::Video => "Video",
            Category::Audio => "Audio",
            Category::Pictures => "Pictures",
            Category::Documents => "Documents",
            Category::Archives => "Archives",
            Category::Executables => "Executables",
            Category::Code => "Code",
            Category::Uncategorized => "Uncategorized",
            Category::Mismatch => "Check manually",
        }
    }

    pub fn from_ext(ext: &str) -> Self {
        let ext = ext.to_ascii_lowercase();
        match ext.as_str() {
            "mismatch" => Category::Mismatch,

            "mp4" | "m4v" | "mov" | "mkv" | "avi" | "webm" | "flv" | "wmv" 
            | "mpg" | "mpeg" | "3gp" | "ogv" | "ts" | "vob" => Category::Video,

            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" 
            | "wma" | "ape" | "alac" | "aiff" | "dsf" | "dsd" => Category::Audio,

            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif"
            | "svg" | "ico" | "heic" | "heif" | "raw" | "cr2" | "nef" 
            | "arw" | "dng" | "psd" | "ai" | "eps" => Category::Pictures,

            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
            | "txt" | "md" | "rtf" | "odt" | "ods" | "odp" 
            | "csv" | "epub" | "mobi" | "djvu" => Category::Documents,

            "zip" | "7z" | "rar" | "gz" | "tar" | "tgz" | "bz2" 
            | "xz" | "zst" | "lz4" | "cab" | "iso" | "dmg" => Category::Archives,

            "exe" | "msi" | "elf" | "app" | "mach-o" | "wasm"
            | "dll" | "so" | "dylib" | "bin" => Category::Executables,

            "rs" | "py" | "js" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp"
            | "java" | "go" | "rb" | "php" | "swift" | "kt" | "cs" | "html" | "css"
            | "scss" | "sass" | "less" | "vue" | "svelte" | "sh" | "bash" | "zsh"
            | "fish" | "ps1" | "bat" | "cmd" | "yaml" | "yml" | "json" | "toml"
            | "xml" | "ini" | "conf" | "config" | "env" | "gitignore" 
            | "dockerfile" | "makefile" | "cmake" | "sql" => Category::Code,

            _ => Category::Uncategorized,
        }
    }
}