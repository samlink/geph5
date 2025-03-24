use native_dialog::MessageType;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::net::TcpStream;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;
use tauri::Manager;
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::SW_SHOWNORMAL;

mod store_cell;
mod prefs;
mod settings;
mod login;

use login::check_login;

const PATH: &str = ".";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![connect, disconnect, is_login, check_login])
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
                    let _ = stop_proxy(Path::new(PATH));
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
fn connect() -> String {
    let command = format!("{PATH}\\geph5-client.exe --config client.yaml");

    let mut child = Command::new("cmd")
        .arg("/C")
        .arg(&command)
        .creation_flags(0x08000000)
        .spawn()
        .expect("Failed to spawn command");

    let mut n = 5;

    while n > 0 {
        if is_proxy_port_open("127.0.0.1", 9910) {
            set_windows_proxy("127.0.0.1:9910");

            // if vpn {
            //     configure_global_proxy_win();
            // }

            break;
        } else {
            n -= 1;
            thread::sleep(Duration::from_secs(1));
        }
    }

    if n <= 0 {
        child.kill().expect("failed to kill child");
        return "failed".to_string();
    }

    "success".to_string()
}

#[tauri::command]
async fn disconnect() -> String {
    stop_proxy(Path::new(PATH)).unwrap();
    return "disconnect".to_string();
}

// // 全局代理
// fn configure_global_proxy_win() {
//     let command = format!(
//         "netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=443 connectaddress=127.0.0.1 connectport=9909"
//     );
//     run_system_command_win(&command);
//     thread::sleep(Duration::from_millis(500)); // 添加延迟确保命令执行完成
// }

// 判断代理端口是否已经开放
fn is_proxy_port_open(proxy_host: &str, proxy_port: u16) -> bool {
    TcpStream::connect((proxy_host, proxy_port)).is_ok()
}

// 执行管理员命令
fn run_system_command_win(command: &str) {
    unsafe {
        let operation_wide: Vec<u16> = OsStr::new("runas")
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect();
        let operation_ptr = operation_wide.as_ptr();

        // 根据命令类型选择正确的程序
        let program = if command.starts_with("netsh") {
            "netsh.exe"
        } else {
            "reg.exe"
        };

        let program_wide: Vec<u16> = OsStr::new(program)
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect();
        let program_ptr = program_wide.as_ptr();

        let command_wide: Vec<u16> = OsString::from(command)
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect();
        let command_ptr = command_wide.as_ptr();

        ShellExecuteW(
            null_mut(),
            operation_ptr,
            program_ptr,
            command_ptr,
            null_mut(),
            SW_SHOWNORMAL,
        );
    }
}

// 关闭 client.exe 且停止代理
fn stop_proxy(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let close_script = path.join("close.bat");
    let output = Command::new("cmd")
        .arg("/C")
        .arg(&close_script)
        .creation_flags(0x08000000)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?; // 等待进程执行完成

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error running close script: {}", stderr).into());
    }

    Ok(())
}

// 设置 http 代理
fn set_windows_proxy(proxy_address: &str) {
    let commands = vec![
        format!(
            "add \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings\" /v ProxyEnable /t REG_DWORD /d 1 /f"
        ),
        format!(
            "add \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings\" /v ProxyServer /t REG_SZ /d {} /f",
            proxy_address
        ),
    ];

    for command in commands {
        run_system_command_win(&command);
        thread::sleep(Duration::from_millis(200));
    }
}
