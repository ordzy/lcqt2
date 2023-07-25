#![feature(try_blocks)]

use std::{env, io};
use std::error::Error;
use std::net::{Ipv4Addr, TcpListener};
use std::path::Path;
use std::process::{Command};
use std::string::String;

use serde_json::json;

use crate::chrome_debugger::ChromeDebugger;

mod chrome_debugger;

fn free_port() -> io::Result<u16> {
    Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?.local_addr()?.port())
}

fn find_lunar_executable() -> Result<String, String> {
    let exe = match env::consts::OS {
        "windows" => env::var("localappdata").or(Err("%localappdata% not defined"))?
            + r"\Programs\lunarclient\Lunar Client.exe",
        "macos" => "/Applications/Lunar Client.app/Contents/MacOS/Lunar Client".into(),
        "linux" => "/usr/bin/lunarclient".into(),
        _ => Err("unsupported os")?
    };

    if !Path::new(&exe).exists() {
        Err(format!("'{}' does not exist", exe))?
    }

    Ok(exe)
}

fn run() -> Result<(), Box<dyn Error>> {
    let lunar_exe = match env::args().nth(1) {
        Some(arg) => arg,
        _ => find_lunar_executable().map_err(|e|
            format!("failed to locate lunars launcher, try passing the path to its executable by argument: {}", e)
        )?
    };

    let port = free_port()?;

    let mut cp = Command::new(lunar_exe)
        .arg(format!("--inspect={}", port))
        .spawn()
        .map_err(|e| format!("failed to start lunar: {}", e))?;

    let res = try {
        let mut debugger = ChromeDebugger::connect(port).map_err(|e| format!("failed to connect debugger: {}", e))?;

        let payload = format!(
            "require(`${{{}}}/gui.asar/main-inject.js`)()",
            serde_json::to_string(env::current_exe()?.parent().unwrap())?
        );

        debugger.send("Runtime.evaluate", json!({
            "expression": payload,
            "includeCommandLineAPI": true
        }))?;
    };

    if let Err(_) = res {
        let _ = cp.kill();
    }

    res
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[error] {}", e);
        std::process::exit(1);
    }
}