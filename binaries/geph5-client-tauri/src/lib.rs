// use native_dialog::MessageType;
use std::thread;
use std::time::Duration;
use tauri::Manager;

mod daemon;
mod login;
mod pac;
mod prefs;
mod settings;
mod store_cell;

use daemon::DAEMON_HANDLE;
use pac::{is_proxy_port_open, set_http_proxy, unset_http_proxy};
// refresh_cell::RefreshCell,
use login::check_login;
use settings::{VPN_MODE, get_config};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            connect,
            disconnect,
            is_login,
            check_login
        ])
        .setup(|app| {
            // if check_running() {
            // native_dialog::MessageDialog::new()
            //     .set_type(MessageType::Error)
            //     .set_text("程序已运行...")
            //     .set_title("Error")
            //     .show_alert()
            //     .unwrap();

            //     exit(0);
            // }

            let main_window = app.get_webview_window("main").unwrap();

            // 监听窗口关闭事件
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let _ = unset_http_proxy().unwrap();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn is_login() -> bool {
    if settings::USERNAME.get().is_empty() || settings::PASSWORD.get().is_empty() {
        return false;
    }

    return true;
}

#[tauri::command]
fn connect(vpn: bool) -> String {
    VPN_MODE.set(vpn);
    DAEMON_HANDLE.start(get_config().unwrap()).unwrap();
    let http_proxy_listen = get_config().unwrap().http_proxy_listen.unwrap();
    let http_proxy = format!("{}", http_proxy_listen);

    let mut n = 5;

    while n > 0 {
        if is_proxy_port_open(&http_proxy) {
            set_http_proxy(http_proxy_listen).unwrap();
            break;
        } else {
            n -= 1;
            thread::sleep(Duration::from_secs(1));
        }
    }

    if n <= 0 {
        DAEMON_HANDLE.stop().unwrap();
        unset_http_proxy().unwrap();
        return "failed".to_string();
    }

    "success".to_string()
}

#[tauri::command]
async fn disconnect() -> String {
    DAEMON_HANDLE.stop().unwrap();
    unset_http_proxy().unwrap();
    return "disconnect".to_string();
}
