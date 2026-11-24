use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use crate::config::app_dirs;
use crate::texts;

mod bundled {
    include!(concat!(env!("OUT_DIR"), "/languages.rs"));
}

// Constants

pub const ENGLISH_CODE: &str = "en";
pub const LANGUAGE_FALLBACK: &str = "language.fallback";
pub const CATEGORY_PREFIX: &str = "# Category:";
pub const STANDARD_CATEGORY: &str = "standard";
const EXTENSION: &str = "lang";
const LANGUAGE_FOLDER: &str = "lang";
const MAX_FILE_SIZE: u64 = 256 * 1024;
const MAX_CODE_LENGTH: usize = 16;
const SIMPLIFIED_CHINESE: &str = "zh-Hans";
const TRADITIONAL_CHINESE: &str = "zh-Hant";
const TRADITIONAL_CHINESE_REGIONS: [&str; 3] = ["TW", "HK", "MO"];
const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

static CURRENT: LazyLock<RwLock<Arc<Translations>>> =
    LazyLock::new(|| RwLock::new(Arc::new(Translations::english())));

// Types

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub code: String,
    pub name: String,
}

pub struct Translations {
    texts: HashMap<String, String>,
}

struct Library {
    files: Vec<(String, String)>,
}

// Current language

pub fn select(preference: Option<&str>) {
    let translations = Translations::load(preference);
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(translations);
}

fn current() -> Arc<Translations> {
    CURRENT
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub fn text(key: &str) -> String {
    current().text(key)
}

pub fn optional_text(key: &str) -> Option<String> {
    current().texts.get(key).cloned()
}

pub fn format_text(key: &str, values: &[(&str, &str)]) -> String {
    current().format(key, values)
}

pub fn available_languages() -> Vec<Language> {
    let library = Library::load(Some(&user_directory()));
    let mut languages: Vec<Language> = library
        .codes()
        .into_iter()
        .map(|code| {
            let name = library
                .texts(&code)
                .remove(texts::LANGUAGE_NAME)
                .unwrap_or_else(|| code.clone());
            Language { code, name }
        })
        .collect();
    languages.sort_by_key(|language| language.name.to_lowercase());
    languages
}

fn user_directory() -> PathBuf {
    app_dirs().config.join(LANGUAGE_FOLDER)
}

// Translations

impl Translations {
    fn english() -> Self {
        Self {
            texts: Library::load(None).texts(ENGLISH_CODE),
        }
    }

    fn load(preference: Option<&str>) -> Self {
        let library = Library::load(Some(&user_directory()));
        let code = resolve_code(
            preference,
            &system_languages(),
            &library.codes(),
            &library.automatic_codes(),
        );
        Self {
            texts: library.texts(&code),
        }
    }

    pub fn text(&self, key: &str) -> String {
        self.texts
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn format(&self, key: &str, values: &[(&str, &str)]) -> String {
        values.iter().fold(self.text(key), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
    }
}

// Library

impl Library {
    fn load(user_directory: Option<&Path>) -> Self {
        let mut files: Vec<(String, String)> = bundled::BUNDLED
            .iter()
            .map(|&(code, content)| (code.to_owned(), content.to_owned()))
            .collect();
        if let Some(directory) = user_directory {
            files.extend(read_directory(directory));
        }
        Self { files }
    }

    fn files_of<'a>(&'a self, code: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.files
            .iter()
            .filter(move |(known, _)| known.eq_ignore_ascii_case(code))
            .map(|(_, content)| content.as_str())
    }

    fn own_texts(&self, code: &str) -> HashMap<String, String> {
        let mut texts = HashMap::new();
        for content in self.files_of(code) {
            texts.extend(parse(content));
        }
        texts
    }

    fn texts(&self, code: &str) -> HashMap<String, String> {
        let fallback = self
            .own_texts(code)
            .remove(LANGUAGE_FALLBACK)
            .filter(|fallback| is_valid_code(fallback));
        let mut texts = HashMap::new();
        for code in language_chain(code, fallback.as_deref()) {
            texts.extend(self.own_texts(code));
        }
        texts
    }

    fn codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = Vec::new();
        for (code, _) in &self.files {
            if !codes.iter().any(|known| known.eq_ignore_ascii_case(code)) {
                codes.push(code.clone());
            }
        }
        codes
    }

    fn automatic_codes(&self) -> Vec<String> {
        self.codes()
            .into_iter()
            .filter(|code| {
                self.files_of(code)
                    .filter_map(category)
                    .last()
                    .is_none_or(|category| category == STANDARD_CATEGORY)
            })
            .collect()
    }
}

fn language_chain<'a>(code: &'a str, fallback: Option<&'a str>) -> Vec<&'a str> {
    let mut chain = Vec::new();
    let candidates = fallback
        .map(fallback_chain)
        .unwrap_or_default()
        .into_iter()
        .chain(fallback_chain(code));
    for candidate in candidates {
        if !chain
            .iter()
            .any(|known: &&str| known.eq_ignore_ascii_case(candidate))
        {
            chain.push(candidate);
        }
    }
    chain
}

fn fallback_chain(code: &str) -> Vec<&str> {
    let mut chain = vec![ENGLISH_CODE];
    for candidate in [base_code(code), code] {
        if !chain
            .iter()
            .any(|known| known.eq_ignore_ascii_case(candidate))
        {
            chain.push(candidate);
        }
    }
    chain
}

fn base_code(code: &str) -> &str {
    code.split('-').next().unwrap_or(code)
}

// Language choice

fn resolve_code(
    preference: Option<&str>,
    system: &[String],
    available: &[String],
    automatic: &[String],
) -> String {
    let find = |list: &[String], code: &str| {
        candidates(code).into_iter().find_map(|candidate| {
            list.iter()
                .find(|known| known.eq_ignore_ascii_case(candidate))
                .cloned()
        })
    };
    preference
        .and_then(|code| find(available, code))
        .or_else(|| system.iter().find_map(|code| find(automatic, code)))
        .unwrap_or_else(|| ENGLISH_CODE.to_owned())
}

fn candidates(code: &str) -> Vec<&str> {
    let mut candidates: Vec<&str> = code
        .char_indices()
        .filter(|&(_, character)| character == '-')
        .map(|(index, _)| &code[..index])
        .rev()
        .collect();
    candidates.insert(0, code);
    if let Some(script) = chinese_script(code) {
        candidates.insert(1, script);
    }
    candidates
}

fn chinese_script(code: &str) -> Option<&'static str> {
    let mut parts = code.split('-');
    if !parts.next()?.eq_ignore_ascii_case("zh") {
        return None;
    }
    let traditional = parts.any(|part| {
        part.eq_ignore_ascii_case("Hant")
            || TRADITIONAL_CHINESE_REGIONS
                .iter()
                .any(|region| part.eq_ignore_ascii_case(region))
    });
    Some(if traditional {
        TRADITIONAL_CHINESE
    } else {
        SIMPLIFIED_CHINESE
    })
}

pub fn is_valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= MAX_CODE_LENGTH
        && code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

// System language

fn system_languages() -> Vec<String> {
    let mut languages = platform_languages();
    languages.extend(locale_languages(|name| std::env::var(name).ok()));
    languages
}

fn locale_languages(var: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut languages: Vec<String> = var("LANGUAGE")
        .unwrap_or_default()
        .split(':')
        .filter_map(posix_locale)
        .collect();
    languages.extend(
        LOCALE_VARIABLES
            .iter()
            .filter_map(|name| var(name))
            .filter_map(|value| posix_locale(&value)),
    );
    languages
}

fn posix_locale(value: &str) -> Option<String> {
    let (locale, modifier) = value.split_once('@').unwrap_or((value, ""));
    let locale = locale.split('.').next().unwrap_or(locale).trim();
    if locale.is_empty() || locale == "C" || locale == "POSIX" {
        return None;
    }
    let mut parts = locale.split('_');
    let mut code = parts.next()?.to_owned();
    if modifier.eq_ignore_ascii_case("latin") {
        code.push_str("-Latn");
    }
    for part in parts {
        code.push('-');
        code.push_str(part);
    }
    is_valid_code(&code).then_some(code)
}

#[cfg(windows)]
fn platform_languages() -> Vec<String> {
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};

    let mut count = 0_u32;
    let mut length = 0_u32;
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &raw mut count,
            std::ptr::null_mut(),
            &raw mut length,
        )
    } == 0
    {
        return Vec::new();
    }
    let mut buffer = vec![0_u16; length as usize];
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &raw mut count,
            buffer.as_mut_ptr(),
            &raw mut length,
        )
    } == 0
    {
        return Vec::new();
    }
    buffer
        .split(|&unit| unit == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

#[cfg(target_os = "macos")]
fn platform_languages() -> Vec<String> {
    use std::ffi::{CStr, c_char, c_void};

    type CFTypeRef = *const c_void;
    type CFIndex = isize;
    const UTF8_ENCODING: u32 = 0x0800_0100;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> CFTypeRef;
        fn CFArrayGetCount(array: CFTypeRef) -> CFIndex;
        fn CFArrayGetValueAtIndex(array: CFTypeRef, index: CFIndex) -> CFTypeRef;
        fn CFStringGetCString(
            string: CFTypeRef,
            buffer: *mut c_char,
            size: CFIndex,
            encoding: u32,
        ) -> u8;
        fn CFRelease(object: CFTypeRef);
    }

    let mut languages = Vec::new();
    unsafe {
        let array = CFLocaleCopyPreferredLanguages();
        if array.is_null() {
            return languages;
        }
        for index in 0..CFArrayGetCount(array) {
            let mut buffer: [c_char; 64] = [0; 64];
            let value = CFArrayGetValueAtIndex(array, index);
            if !value.is_null()
                && CFStringGetCString(value, buffer.as_mut_ptr(), 64, UTF8_ENCODING) != 0
            {
                languages.push(
                    CStr::from_ptr(buffer.as_ptr())
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        CFRelease(array);
    }
    languages
}

#[cfg(not(any(windows, target_os = "macos")))]
fn platform_languages() -> Vec<String> {
    Vec::new()
}

// Files

fn read_directory(directory: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut files: Vec<(String, String)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
        })
        .filter_map(|path| {
            let code = path.file_stem()?.to_str()?.to_owned();
            if !is_valid_code(&code) {
                return None;
            }
            Some((code, read_file(&path)?))
        })
        .collect();
    files.sort();
    files
}

fn read_file(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE {
        return None;
    }
    fs::read_to_string(path).ok()
}

pub fn category(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.strip_prefix(CATEGORY_PREFIX))
        .map(|category| category.trim().to_owned())
}

pub fn parse(content: &str) -> HashMap<String, String> {
    content
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with(';'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .map(|(key, value)| (key.to_owned(), value.replace("\\n", "\n")))
        .collect()
}

// Tests

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::keys::KeyboardLayout;

    const LANGUAGE_DIRECTION: &str = "language.direction";
    const OPTIONAL_KEYS: [&str; 2] = [LANGUAGE_FALLBACK, LANGUAGE_DIRECTION];
    const DIRECTIONS: [&str; 2] = ["ltr", "rtl"];

    fn codes(values: &[&str]) -> Vec<String> {
        values.iter().map(|&value| value.to_owned()).collect()
    }

    fn placeholders(text: &str) -> BTreeSet<String> {
        text.split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}'))
            .map(|(name, _)| name.to_owned())
            .collect()
    }

    fn temp_directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("flexpad-lang-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn is_layout_name(key: &str) -> bool {
        KeyboardLayout::ALL
            .iter()
            .any(|layout| layout.text_key() == key)
    }

    fn english() -> HashMap<String, String> {
        Library::load(None).texts(ENGLISH_CODE)
    }

    // Parsing

    #[test]
    fn parses_language_files() {
        let texts = parse(
            "\u{feff}# Comment\n; Comment\nmenu.quit = Quit \nbroken\nempty =\nmulti = One\\nTwo\n",
        );
        assert_eq!(texts.len(), 2);
        assert_eq!(texts["menu.quit"], "Quit");
        assert_eq!(texts["multi"], "One\nTwo");
    }

    #[test]
    fn formats_placeholders() {
        let translations = Translations {
            texts: parse("greeting = Hello {name}, {name}!"),
        };
        assert_eq!(
            translations.format("greeting", &[("name", "Ada")]),
            "Hello Ada, Ada!"
        );
        assert_eq!(translations.text("missing"), "missing");
    }

    // Language choice

    #[test]
    fn resolves_the_language() {
        let available = codes(&["en", "de", "pt-BR"]);
        let resolve = |preference, system: &[&str]| {
            resolve_code(preference, &codes(system), &available, &available)
        };
        assert_eq!(resolve(None, &["de-AT", "en-US"]), "de");
        assert_eq!(resolve(None, &["pt-br"]), "pt-BR");
        assert_eq!(resolve(None, &["fr-FR"]), "en");
        assert_eq!(resolve(Some("en"), &["de-DE"]), "en");
        assert_eq!(resolve(Some("xx"), &["de-DE"]), "de");
    }

    #[test]
    fn dialects_and_fun_languages_are_never_chosen_automatically() {
        let available = codes(&["en", "de", "nds", "en-x-pirate"]);
        let automatic = codes(&["en", "de"]);
        assert_eq!(
            resolve_code(None, &codes(&["nds-DE", "de-DE"]), &available, &automatic),
            "de"
        );
        assert_eq!(
            resolve_code(Some("nds"), &codes(&["de-DE"]), &available, &automatic),
            "nds"
        );
        let library = Library::load(None);
        let automatic = library.automatic_codes();
        assert!(automatic.iter().any(|code| code == "de"));
        assert!(!automatic.iter().any(|code| code == "bar"));
        assert!(!automatic.iter().any(|code| code == "en-x-pirate"));
    }

    #[test]
    fn shortens_system_language_codes() {
        let available = codes(&["en", "sr-Latn", "nb"]);
        let resolve = |system: &[&str]| resolve_code(None, &codes(system), &available, &available);
        assert_eq!(resolve(&["sr-Latn-RS"]), "sr-Latn");
        assert_eq!(resolve(&["nb-NO"]), "nb");
        assert_eq!(candidates("sr-Latn-RS"), ["sr-Latn-RS", "sr-Latn", "sr"]);
        assert_eq!(candidates("zh-TW"), ["zh-TW", "zh-Hant", "zh"]);
    }

    #[test]
    fn maps_chinese_regions_to_scripts() {
        let available = codes(&["en", "zh-Hans", "zh-Hant"]);
        let resolve = |system: &[&str]| resolve_code(None, &codes(system), &available, &available);
        assert_eq!(resolve(&["zh-CN"]), "zh-Hans");
        assert_eq!(resolve(&["zh-SG"]), "zh-Hans");
        assert_eq!(resolve(&["zh-TW"]), "zh-Hant");
        assert_eq!(resolve(&["zh-HK"]), "zh-Hant");
        assert_eq!(resolve(&["zh-Hant-TW"]), "zh-Hant");
        assert_eq!(chinese_script("de-DE"), None);
    }

    #[test]
    fn reads_posix_locales() {
        let variables = HashMap::from([
            ("LANGUAGE", "de_AT:fr"),
            ("LC_ALL", "C"),
            ("LC_MESSAGES", "sr_RS.UTF-8@latin"),
            ("LANG", "pt_BR.UTF-8"),
        ]);
        assert_eq!(
            locale_languages(|name| variables.get(name).map(|value| (*value).to_owned())),
            ["de-AT", "fr", "sr-Latn-RS", "pt-BR"]
        );
        assert_eq!(posix_locale("POSIX"), None);
        assert_eq!(posix_locale(""), None);
        assert_eq!(locale_languages(|_| None), Vec::<String>::new());
    }

    // Fallback

    #[test]
    fn falls_back_through_english() {
        assert_eq!(fallback_chain("de-AT"), ["en", "de", "de-AT"]);
        assert_eq!(fallback_chain("de"), ["en", "de"]);
        assert_eq!(fallback_chain("en"), ["en"]);
    }

    #[test]
    fn falls_back_through_the_declared_language() {
        assert_eq!(language_chain("sxu", Some("de")), ["en", "de", "sxu"]);
        assert_eq!(
            language_chain("de-x-amtsdt", Some("de")),
            ["en", "de", "de-x-amtsdt"]
        );
        assert_eq!(language_chain("de-CH", None), ["en", "de", "de-CH"]);
    }

    #[test]
    fn falls_back_per_text() {
        let directory = temp_directory("per-text");
        fs::write(directory.join("xx.lang"), "device.scan = Search now\n").unwrap();
        fs::write(
            directory.join("yy.lang"),
            "language.fallback = de\ndevice.scan = Gugge\n",
        )
        .unwrap();
        let library = Library::load(Some(&directory));
        fs::remove_dir_all(&directory).unwrap();
        let english = english();
        let german = library.texts("de");

        let texts = library.texts("xx");
        assert_eq!(texts[texts::DEVICE_SCAN], "Search now");
        assert_eq!(texts[texts::DEVICE_CONNECT], english[texts::DEVICE_CONNECT]);

        let texts = library.texts("yy");
        assert_eq!(texts[texts::DEVICE_SCAN], "Gugge");
        assert_eq!(texts[texts::DEVICE_CONNECT], german[texts::DEVICE_CONNECT]);
    }

    #[test]
    fn user_files_override_single_texts_and_add_languages() {
        let directory = temp_directory("user");
        fs::write(directory.join("de.lang"), "device.scan = Gerät suchen\n").unwrap();
        fs::write(directory.join("eo.lang"), "language.name = Esperanto\n").unwrap();
        fs::write(directory.join("bad name.lang"), "language.name = Bad\n").unwrap();
        let library = Library::load(Some(&directory));
        fs::remove_dir_all(&directory).unwrap();
        let bundled = Library::load(None).texts("de");

        let texts = library.texts("de");
        assert_eq!(texts[texts::DEVICE_SCAN], "Gerät suchen");
        assert_eq!(texts[texts::DEVICE_CONNECT], bundled[texts::DEVICE_CONNECT]);
        assert!(library.codes().iter().any(|code| code == "eo"));
        assert!(library.automatic_codes().iter().any(|code| code == "eo"));
        assert!(!library.codes().iter().any(|code| code == "bad name"));
    }

    // Bundled files

    #[test]
    fn english_is_bundled_and_complete() {
        let english = english();
        assert_eq!(english[texts::LANGUAGE_NAME], "English");
        assert_eq!(
            Translations::english().text(texts::DEVICE_SCAN),
            english[texts::DEVICE_SCAN]
        );
    }

    #[test]
    fn bundled_languages_match_english() {
        let english = english();
        for &(code, content) in bundled::BUNDLED {
            assert!(is_valid_code(code), "{code} has an invalid name");
            let texts = parse(content);
            for (key, text) in &texts {
                if OPTIONAL_KEYS.contains(&key.as_str()) || is_layout_name(key) {
                    continue;
                }
                let Some(original) = english.get(key) else {
                    panic!("{code} contains the unknown text {key}");
                };
                assert_eq!(
                    placeholders(text),
                    placeholders(original),
                    "{code}: {key} has other placeholders"
                );
            }
            for key in english.keys() {
                assert!(texts.contains_key(key), "{code} is missing {key}");
            }
            if let Some(fallback) = texts.get(LANGUAGE_FALLBACK) {
                assert!(
                    bundled::BUNDLED.iter().any(|(known, _)| known == fallback),
                    "{code} falls back to the unknown language {fallback}"
                );
            }
            if let Some(direction) = texts.get(LANGUAGE_DIRECTION) {
                assert!(
                    DIRECTIONS.contains(&direction.as_str()),
                    "{code} has the unknown direction {direction}"
                );
            }
        }
    }

    #[test]
    fn language_names_are_unique() {
        let library = Library::load(None);
        let mut names = BTreeSet::new();
        for code in library.codes() {
            let name = library.texts(&code).remove(texts::LANGUAGE_NAME).unwrap();
            assert!(names.insert(name.clone()), "{code}: {name} is used twice");
        }
    }
}
