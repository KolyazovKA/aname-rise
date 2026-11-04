// управление снаружи: `amane-shell toggle clipboard` шлёт команду уже запущенному бару через unix-сокет
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use crate::state::{Popup, Ui};

fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(dir).join("amane-shell.sock")
}

// клиент: true, если команду приняли
pub fn send(command: &str) -> bool {
    UnixStream::connect(socket_path())
        .and_then(|mut stream| stream.write_all(format!("{command}\n").as_bytes()))
        .is_ok()
}

// сервер: слушает сокет в отдельном потоке
pub fn serve() {
    let path = socket_path();

    // остаток от прошлого запуска
    let _ = fs::remove_file(&path);

    let Ok(listener) = UnixListener::bind(&path) else {
        return;
    };

    // команды принимаем только от своего пользователя
    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();

            if BufReader::new(stream).read_line(&mut line).is_ok() {
                handle(line.trim());
            }
        }
    });
}

fn handle(command: &str) {
    if command == "toggle clipboard" {
        toggle(Popup::Clipboard);
    }
}

// окно открывается на мониторе, где сейчас фокус
fn toggle(popup: Popup) {
    let Some(monitor) = focused_output() else {
        return;
    };

    Ui::toggle(popup, &monitor);
}

fn focused_output() -> Option<String> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-output"])
        .output()
        .ok()?;

    json_string_field(&String::from_utf8(output.stdout).ok()?, "name")
}

// первое строковое поле с таким именем; для одного имени монитора хватает, парсер JSON не нужен
fn json_string_field(json: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let rest = json[json.find(&marker)? + marker.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start().strip_prefix('"')?;

    let mut value = String::new();
    let mut chars = rest.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(value),
            '\\' => value.push(chars.next()?),
            _ => value.push(ch),
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::json_string_field;

    #[test]
    fn reads_output_name() {
        let json = r#"{"name":"eDP-1","make":"BOE","modes":[{"width":1920}]}"#;

        assert_eq!(json_string_field(json, "name").as_deref(), Some("eDP-1"));
    }

    #[test]
    fn tolerates_spaces_and_escapes() {
        let json = r#"{ "name" : "DP\"1" }"#;

        assert_eq!(json_string_field(json, "name").as_deref(), Some("DP\"1"));
    }

    #[test]
    fn missing_or_broken_gives_none() {
        assert_eq!(json_string_field("null", "name"), None);
        assert_eq!(json_string_field(r#"{"name":null}"#, "name"), None);
        assert_eq!(json_string_field(r#"{"name":"abc"#, "name"), None);
    }
}