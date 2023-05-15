use std::{
    eprintln,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use sysinfo::{ProcessExt, System, SystemExt};

use crate::{
    kill_process, refresh_sysinfo, start_process, AlreadyStartedCommands, CommandExecutionResult,
    OptionFunc,
};

pub fn create_minikube_tunnel(
    already_started_commands: AlreadyStartedCommands,
    sysinfo: Arc<Mutex<System>>,
) -> Result<(), String> {
    if let Ok(false) = check_minikube_tunnel(Arc::clone(&sysinfo)) {
        let result = toggle_minikube_tunnel(
            false,
            Arc::clone(&sysinfo),
            Arc::clone(&already_started_commands),
        );

        return match result {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        };
    }

    Ok(())
}

pub fn build_minikube_tunnel_option(
    already_started_commands: AlreadyStartedCommands,
    sysinfo: Arc<Mutex<System>>,
) -> Result<(String, OptionFunc), String> {
    let check_minikube_tunnel_result = check_minikube_tunnel(Arc::clone(&sysinfo));

    match check_minikube_tunnel_result {
        Ok(running) => {
            let next_state = if running { "Stop" } else { "Start" };

            Ok((
                format!("{next_state} minikube tunnel"),
                Box::new(move || {
                    toggle_minikube_tunnel(
                        running,
                        Arc::clone(&sysinfo),
                        Arc::clone(&already_started_commands),
                    )
                }),
            ))
        }
        Err(error) => Err(format!("Error in build_minikube_tunnel_option: {error}")),
    }
}

fn check_minikube_tunnel(sysinfo: Arc<Mutex<System>>) -> Result<bool, String> {
    let sysinfo = refresh_sysinfo(&sysinfo);

    let found = sysinfo
        .processes_by_name("minikube")
        .any(|x| x.cmd().join(" ").contains("tunnel"));

    Ok(found)
}

fn toggle_minikube_tunnel(
    running: bool,
    sysinfo: Arc<Mutex<System>>,
    already_started_commands: AlreadyStartedCommands,
) -> CommandExecutionResult {
    if running {
        let result = kill_process("minikube", "tunnel", Arc::clone(&sysinfo));

        println!("minikube tunnel has been stopped\n");

        result
    } else {
        let result = start_process(
            "minikube",
            &["tunnel", "--bind-address", "127.0.0.1"],
            Some(String::from("Could not proxy minikube tunnel")),
            already_started_commands,
        )?;

        {
            let mut cnt = 0;

            while !check_minikube_tunnel(Arc::clone(&sysinfo))? && cnt < 5 {
                cnt += 1;
                thread::sleep(Duration::from_secs(1));
            }

            if cnt == 5 {
                return Err(String::from(
                    "Failed to verify if minikube tunnel has been started",
                ));
            }
        }

        println!("minikube tunnel has been started\n");

        Ok(result)
    }
}
