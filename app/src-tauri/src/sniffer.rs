use tauri::AppHandle;
/// Relaunches Nous elevated. Waits for the UAC answer: on "Yes" this instance
/// quits (the new one waits for that, see `wait_for_previous`), on "No" it
/// keeps running and resolves to false.
#[tauri::command]
pub async fn restart_as_admin(app: AppHandle) -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = format!(
        "Start-Process -FilePath {} -ArgumentList '{}','{}' -Verb RunAs",
        crate::launch::ps_quote(&exe.display().to_string()),
        WAIT_PID_ARG,
        std::process::id()
    );
    let started = tauri::async_runtime::spawn_blocking(move || crate::launch::shell_start(&command))
        .await
        .map_err(|e| e.to_string())??;
    if started {
        app.exit(0);
    }
    Ok(started)
}

/// Argument of an elevated restart: the pid of the instance being replaced.
pub const WAIT_PID_ARG: &str = "--wait-pid";

/// Called first thing at startup: after an elevated restart, waits (up to 15 s)
/// for the old instance to quit, so the single-instance guard doesn't send
/// this new one away.
pub fn wait_for_previous() {
    let args: Vec<String> = std::env::args().collect();
    let Some(pid) = args.iter().position(|a| a == WAIT_PID_ARG).and_then(|i| args.get(i + 1)).and_then(|p| p.parse::<u32>().ok())
    else {
        return;
    };
    let pid = sysinfo::Pid::from_u32(pid);
    let mut sys = sysinfo::System::new();
    for _ in 0..150 {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        if sys.process(pid).is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
