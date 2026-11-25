use std::path::{Path, PathBuf};
use std::process::Command;

// Constants

const JSON_EXTENSION: &str = "json";

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    Open,
    Save,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogRequest {
    pub kind: DialogKind,
    pub title: String,
    pub filter_name: String,
    pub file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(windows, allow(dead_code))]
pub enum DialogError {
    Unavailable,
    Failed(String),
}

pub type DialogResult = Result<Option<PathBuf>, DialogError>;

// Paths

pub fn with_json_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(JSON_EXTENSION)
    }
}

// Folder

pub fn open_folder(path: &Path) -> Result<(), String> {
    folder_command(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn folder_command(path: &Path) -> Command {
    let mut command = Command::new("explorer");
    command.arg(path);
    command
}

#[cfg(target_os = "macos")]
fn folder_command(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg(path);
    command
}

#[cfg(all(unix, not(target_os = "macos")))]
fn folder_command(path: &Path) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    command
}

// Windows

#[cfg(windows)]
pub fn pick(request: &DialogRequest, owner: Option<u64>) -> DialogResult {
    use windows_sys::Win32::UI::Controls::Dialogs::{
        CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
        OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };

    const MAX_PATH_CHARS: usize = 32_768;

    let filter = windows_filter(&request.filter_name);
    let title = wide(&request.title);
    let default_extension = wide(JSON_EXTENSION);
    let mut buffer = vec![0u16; MAX_PATH_CHARS];
    if request.kind == DialogKind::Save {
        let name: Vec<u16> = request.file_name.encode_utf16().collect();
        let length = name.len().min(MAX_PATH_CHARS - 1);
        buffer[..length].copy_from_slice(&name[..length]);
    }
    let kind_flags = match request.kind {
        DialogKind::Open => OFN_FILEMUSTEXIST,
        DialogKind::Save => OFN_OVERWRITEPROMPT,
    };
    let mut dialog = OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner.map_or(std::ptr::null_mut(), |owner| owner as _),
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: buffer.as_mut_ptr(),
        nMaxFile: MAX_PATH_CHARS as u32,
        lpstrTitle: title.as_ptr(),
        lpstrDefExt: default_extension.as_ptr(),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_HIDEREADONLY | kind_flags,
        ..OPENFILENAMEW::default()
    };
    let accepted = unsafe {
        match request.kind {
            DialogKind::Open => GetOpenFileNameW(&mut dialog),
            DialogKind::Save => GetSaveFileNameW(&mut dialog),
        }
    } != 0;
    if accepted {
        return Ok(Some(PathBuf::from(from_wide(&buffer))));
    }
    match unsafe { CommDlgExtendedError() } {
        0 => Ok(None),
        code => Err(DialogError::Failed(format!("0x{code:04X}"))),
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(windows)]
fn windows_filter(filter_name: &str) -> Vec<u16> {
    let name = filter_name.replace('\0', "");
    format!("{name} (*.json)\0*.json\0\0")
        .encode_utf16()
        .collect()
}

// macOS

#[cfg(target_os = "macos")]
pub fn pick(request: &DialogRequest, _owner: Option<u64>) -> DialogResult {
    let output = Command::new("osascript")
        .args(osascript_args(request))
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => DialogError::Unavailable,
            _ => DialogError::Failed(error.to_string()),
        })?;
    if output.status.success() {
        return Ok(chosen_path(&output.stdout));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("(-128)") {
        Ok(None)
    } else {
        Err(DialogError::Failed(stderr.trim().to_string()))
    }
}

#[cfg(target_os = "macos")]
fn osascript_args(request: &DialogRequest) -> Vec<String> {
    let choose = match request.kind {
        DialogKind::Open => r#"choose file with prompt (item 1 of argv) of type {"public.json"}"#,
        DialogKind::Save => {
            "choose file name with prompt (item 1 of argv) default name (item 2 of argv)"
        }
    };
    [
        "-e".to_string(),
        "on run argv".to_string(),
        "-e".to_string(),
        format!("POSIX path of ({choose})"),
        "-e".to_string(),
        "end run".to_string(),
        request.title.clone(),
        request.file_name.clone(),
    ]
    .into()
}

// Linux

#[cfg(all(unix, not(target_os = "macos")))]
pub fn pick(request: &DialogRequest, _owner: Option<u64>) -> DialogResult {
    let prefers_kdialog = std::env::var("XDG_CURRENT_DESKTOP")
        .is_ok_and(|desktop| desktop.to_ascii_uppercase().contains("KDE"));
    let mut tools = [
        ("zenity", zenity_args(request)),
        ("kdialog", kdialog_args(request)),
    ];
    if prefers_kdialog {
        tools.reverse();
    }
    for (program, args) in tools {
        match Command::new(program).args(args).output() {
            Ok(output) if output.status.success() => return Ok(chosen_path(&output.stdout)),
            Ok(output) if output.status.code() == Some(1) => return Ok(None),
            Ok(output) => {
                return Err(DialogError::Failed(
                    String::from_utf8_lossy(&output.stderr).trim().to_string(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(DialogError::Failed(error.to_string())),
        }
    }
    Err(DialogError::Unavailable)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn zenity_args(request: &DialogRequest) -> Vec<String> {
    let mut args = vec![
        "--file-selection".to_string(),
        format!("--title={}", request.title),
        format!("--file-filter={} | *.json", request.filter_name),
    ];
    if request.kind == DialogKind::Save {
        args.push("--save".to_string());
        args.push("--confirm-overwrite".to_string());
        args.push(format!("--filename={}", request.file_name));
    }
    args
}

#[cfg(all(unix, not(target_os = "macos")))]
fn kdialog_args(request: &DialogRequest) -> Vec<String> {
    let (mode, start) = match request.kind {
        DialogKind::Open => ("--getopenfilename", "."),
        DialogKind::Save => ("--getsavefilename", request.file_name.as_str()),
    };
    vec![
        "--title".to_string(),
        request.title.clone(),
        mode.to_string(),
        start.to_string(),
        format!("*.json|{}", request.filter_name),
    ]
}

#[cfg(unix)]
fn chosen_path(stdout: &[u8]) -> Option<PathBuf> {
    let path = String::from_utf8_lossy(stdout);
    let path = path.trim_end_matches(['\r', '\n']);
    (!path.is_empty()).then(|| PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn request(kind: DialogKind) -> DialogRequest {
        DialogRequest {
            kind,
            title: "Export profile".to_string(),
            filter_name: "FlexPad profiles".to_string(),
            file_name: "Desk.json".to_string(),
        }
    }

    // Paths

    #[test]
    fn adds_json_only_without_an_extension() {
        assert_eq!(
            with_json_extension(PathBuf::from("backup")),
            PathBuf::from("backup.json")
        );
        assert_eq!(
            with_json_extension(PathBuf::from("backup.txt")),
            PathBuf::from("backup.txt")
        );
        assert_eq!(
            with_json_extension(PathBuf::from("Desk.json")),
            PathBuf::from("Desk.json")
        );
    }

    // Windows

    #[cfg(windows)]
    #[test]
    fn windows_filter_is_double_nul_terminated() {
        let filter = String::from_utf16(&windows_filter("Profiles\0x")).unwrap();

        assert_eq!(filter, "Profilesx (*.json)\0*.json\0\0");
    }

    #[cfg(windows)]
    #[test]
    fn wide_strings_round_trip() {
        let mut buffer = wide("C:\\Größe\\Desk.json");
        buffer.extend([0, 65, 66]);

        assert_eq!(from_wide(&buffer), "C:\\Größe\\Desk.json");
        assert_eq!(*wide("a").last().unwrap(), 0);
    }

    // macOS

    #[cfg(target_os = "macos")]
    #[test]
    fn osascript_passes_texts_as_arguments() {
        let save = osascript_args(&request(DialogKind::Save));
        let open = osascript_args(&request(DialogKind::Open));

        assert_eq!(&save[6..], ["Export profile", "Desk.json"]);
        assert!(save[3].contains("choose file name"));
        assert!(open[3].contains("public.json"));
        assert!(!save[3].contains("Export profile"));
    }

    // Linux

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn zenity_asks_before_overwriting() {
        let save = zenity_args(&request(DialogKind::Save));
        let open = zenity_args(&request(DialogKind::Open));

        assert!(save.contains(&"--save".to_string()));
        assert!(save.contains(&"--confirm-overwrite".to_string()));
        assert!(save.contains(&"--filename=Desk.json".to_string()));
        assert!(!open.contains(&"--save".to_string()));
        assert!(open.contains(&"--file-filter=FlexPad profiles | *.json".to_string()));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn kdialog_uses_open_and_save_modes() {
        let save = kdialog_args(&request(DialogKind::Save));
        let open = kdialog_args(&request(DialogKind::Open));

        assert_eq!(save[2..4], ["--getsavefilename", "Desk.json"]);
        assert_eq!(open[2], "--getopenfilename");
        assert_eq!(open[4], "*.json|FlexPad profiles");
    }

    #[cfg(unix)]
    #[test]
    fn reads_the_chosen_path_from_stdout() {
        assert_eq!(
            chosen_path(b"/home/u/Desk.json\n"),
            Some(PathBuf::from("/home/u/Desk.json"))
        );
        assert_eq!(chosen_path(b"\n"), None);
    }
}
