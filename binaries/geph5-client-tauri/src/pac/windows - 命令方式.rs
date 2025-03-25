use std::process::Command;
use std::os::windows::process::CommandExt;
use anyhow::Context;
use std::net::TcpStream;

fn run_system_command_win(command_args: &[&str]) -> anyhow::Result<()> {
    let _ = Command::new("C:\\Windows\\System32\\reg.exe")
        .args(command_args)
        .creation_flags(0x08000000)
        .output()
        .context("Failed to execute reg command")?;

    Ok(())
}

pub fn set_http_proxy(proxy_address: &str) -> anyhow::Result<()> {
    let enable_proxy = [
        "add",
        "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
        "/v", "ProxyEnable",
        "/t", "REG_DWORD",
        "/d", "1",
        "/f",
    ];
    let set_proxy_server = [
        "add",
        "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
        "/v", "ProxyServer",
        "/t", "REG_SZ",
        "/d", proxy_address,
        "/f",
    ];

    run_system_command_win(&enable_proxy)?;
    run_system_command_win(&set_proxy_server)?;

    Ok(())
}

pub fn unset_http_proxy() -> anyhow::Result<()> {
    let disable_proxy = [
        "add",
        "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
        "/v", "ProxyEnable",
        "/t", "REG_DWORD",
        "/d", "0",
        "/f",
    ];
    run_system_command_win(&disable_proxy)?;
    Ok(())
}

pub fn is_proxy_port_open(proxy_address: &str) -> bool {
    let proxy = proxy_address.split(":").collect::<Vec<&str>>();
    TcpStream::connect((proxy[0], proxy[1].parse::<u16>().unwrap())).is_ok()
}
