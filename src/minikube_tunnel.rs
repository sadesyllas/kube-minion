use std::{thread, time::Duration};

use sysinfo::{ProcessExt, SystemExt};

use crate::{
    get_sys_info, kill_process, print_results, start_and_wait_process, CommandExecutionResult,
    CommandResultType::*, OptionFunc,
};

pub fn create_minikube_tunnel() -> CommandExecutionResult {
    if let Ok(true) = check_minikube_tunnel() {
        toggle_minikube_tunnel(true)?;
    }

    toggle_minikube_tunnel(false)?;

    Ok(PrintableResults(
        None,
        vec![String::from("The minikube tunnel has been started")],
    ))
}

pub fn stop_minikube_tunnel() -> CommandExecutionResult {
    if let Ok(true) = check_minikube_tunnel() {
        toggle_minikube_tunnel(true)
    } else {
        Ok(PrintableResults(None, Vec::new()))
    }
}

pub fn build_minikube_tunnel_option() -> Result<(String, OptionFunc, bool), String> {
    match check_minikube_tunnel() {
        Ok(running) => {
            let next_state = if running { "Stop" } else { "Start" };

            Ok((
                format!("{next_state} minikube tunnel"),
                Box::new(move || toggle_minikube_tunnel(running)),
                false,
            ))
        }
        Err(error) => Err(format!("Error in build_minikube_tunnel_option: {error}")),
    }
}

fn check_minikube_tunnel() -> Result<bool, String> {
    Ok(get_sys_info()
        .processes_by_name("minikube")
        .any(|x| x.cmd().join(" ").contains("tunnel")))
}

fn toggle_minikube_tunnel(running: bool) -> CommandExecutionResult {
    if running {
        if let error @ Err(_) = kill_process("minikube", vec!["tunnel"]) {
            return error;
        }

        clear_minikube_ssh_tunnels()?;

        Ok(PrintableResults(
            None,
            vec![String::from("The minikube tunnel has been stopped")],
        ))
    } else {
        clear_minikube_ssh_tunnels()?;

        thread::spawn(move || {
            print_results(
                start_and_wait_process(
                    "minikube",
                    &["tunnel", "-c", "--bind-address=127.0.0.1"],
                    Some(String::from("Failed to start the minikube tunnel")),
                ),
                false,
                true,
            );
        });

        {
            let mut cnt = 0;

            while !check_minikube_tunnel()? && cnt < 5 {
                cnt += 1;
                thread::sleep(Duration::from_secs(1));
            }

            if cnt == 5 {
                return Err(String::from(
                    "Failed to verify if minikube tunnel has been started",
                ));
            }
        }

        Ok(PrintableResults(
            None,
            vec![String::from("The minikube tunnel has been started")],
        ))
    }
}

fn clear_minikube_ssh_tunnels() -> Result<(), String> {
    let sys_info = get_sys_info();
    let ssh_processes = sys_info.processes_by_name("ssh").filter(|x| {
        let cmd = x.cmd().join(" ");

        cmd.contains("docker@127.0.0.1")
            && cmd.contains("minikube/id_rsa")
            && cmd.contains("-L 127.0.0.1:")
    });

    for ssh_process in ssh_processes {
        if !ssh_process.kill() {
            return Err(format!(
                "Failed to kill minikube ssh tunnel with process id {pid}",
                pid = ssh_process.pid(),
            ));
        }
    }

    Ok(())
}
