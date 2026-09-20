use std::fs;
use std::io;
use std::path::Path;

use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider};

pub struct FontFile {
    pub name: String,
    pub display_name: String,
    pub data: Vec<u8>,
}

pub fn load_fonts() -> io::Result<Vec<FontFile>> {
    let mut paths = fs::read_dir(Path::new("resources").join("fonts"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        let extension = extension.to_ascii_lowercase();
                        matches!(extension.as_str(), "otc" | "otf" | "ttc" | "ttf")
                            || cfg!(feature = "woff")
                                && matches!(extension.as_str(), "woff" | "woff2")
                    })
        })
        .collect::<Vec<_>>();
    paths.sort_by_cached_key(|path| {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase()
    });

    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let data = fs::read(path)?;
            Ok(FontFile {
                display_name: font_display_name(&data).unwrap_or_else(|| name.clone()),
                name,
                data,
            })
        })
        .collect()
}

fn font_display_name(data: &[u8]) -> Option<String> {
    #[cfg(feature = "woff")]
    let decoded;
    #[cfg(feature = "woff")]
    let data = match data.get(..4) {
        Some(b"wOFF") => {
            decoded = wuff::decompress_woff1(data).ok()?;
            decoded.as_slice()
        }
        Some(b"wOF2") => {
            decoded = wuff::decompress_woff2(data).ok()?;
            decoded.as_slice()
        }
        _ => data,
    };

    let font = FontRef::from_index(data, 0).ok()?;
    let names = [4, 16, 1]
        .into_iter()
        .flat_map(|name_id| {
            font.localized_strings(StringId::new(name_id))
                .map(|name| name.to_string())
        })
        .filter(|name| !name.trim().is_empty())
        .collect::<Vec<_>>();
    names
        .iter()
        .find(|name| name.chars().any(is_chinese_character))
        .cloned()
        .or_else(|| names.into_iter().next())
}

fn is_chinese_character(character: char) -> bool {
    matches!(
        character,
        '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}'
    )
}
