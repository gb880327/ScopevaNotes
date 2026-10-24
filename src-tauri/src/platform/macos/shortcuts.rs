use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::models::{AppError, AppResult};
use crate::state::AppState;

/// `CommandOrControl` resolves to Command on macOS, so this is ⌘⇧D.
pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+D";

fn parse(accelerator: &str) -> AppResult<Shortcut> {
    let shortcut: Shortcut = accelerator
        .trim()
        .parse()
        .map_err(|_| AppError::Invalid(format!("无法识别的快捷键：{accelerator}")))?;
    if shortcut.mods.is_empty() {
        return Err(AppError::Invalid(
            "全局快捷键需要至少一个修饰键（⌘/⌃/⌥/⇧）".into(),
        ));
    }
    Ok(shortcut)
}

/// Registers the startup accelerator. A taken shortcut must not abort startup.
pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> AppResult<Shortcut> {
    let shortcut = parse(accelerator)?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|err| AppError::Conflict(format!("快捷键注册失败：{err}")))?;
    Ok(shortcut)
}

/// Replaces the active global shortcut. On failure the previous accelerator is
/// re-registered, so the user never ends up with no working shortcut.
pub fn apply<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    accelerator: &str,
) -> AppResult<String> {
    let next = parse(accelerator)?;
    let previous = state.shortcut();
    if let Ok(current) = previous.parse::<Shortcut>() {
        if current.id() == next.id() {
            return Ok(previous);
        }
        let _ = app.global_shortcut().unregister(current);
    }

    match app.global_shortcut().register(next) {
        Ok(()) => {
            state.set_shortcut(accelerator.trim());
            Ok(state.shortcut())
        }
        Err(err) => {
            if let Ok(restore) = previous.parse::<Shortcut>() {
                let _ = app.global_shortcut().register(restore);
            }
            Err(AppError::Conflict(format!(
                "快捷键不可用（可能已被其他应用占用）：{err}"
            )))
        }
    }
}

/// Called from the plugin handler for every registered accelerator.
pub fn on_pressed<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(err) = state.window.toggle(app) {
            eprintln!("[window] toggle failed: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_command_shift_d() {
        let parsed = parse(DEFAULT_SHORTCUT).expect("default parses");
        assert!(parsed.mods.contains(tauri_plugin_global_shortcut::Modifiers::SUPER));
        assert!(parsed.mods.contains(tauri_plugin_global_shortcut::Modifiers::SHIFT));
        assert_eq!(parsed.key, tauri_plugin_global_shortcut::Code::KeyD);
    }

    #[test]
    fn accepts_letter_digit_and_function_keys() {
        for accelerator in ["Command+Alt+K", "CommandOrControl+Shift+1", "Control+F5", "Super+Space"] {
            assert!(parse(accelerator).is_ok(), "{accelerator} should parse");
        }
    }

    #[test]
    fn rejects_garbage_and_bare_keys() {
        assert!(parse("").is_err());
        assert!(parse("not+a+shortcut").is_err());
        assert!(parse("D").is_err(), "bare key has no modifier");
    }
}
