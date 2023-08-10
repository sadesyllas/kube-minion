use std::{thread, time::Duration};

use sysinfo::{ProcessExt, SystemExt};

use crate::{
    get_sys_info, kill_process, merge_if_ok, parse_string, print_results, start_and_wait_process,
    CommandExecutionResult, CommandResultType::*, OptionFunc,
};

static mut BIND_ADDRESS: Option<String> = None;

pub fn create_minikube_tunnel() -> CommandExecutionResult {
    if let Ok(true) = check_minikube_tunnel() {
        toggle_minikube_tunnel(true)?;
    }

    toggle_minikube_tunnel(false)?;

    Ok(PrintableResults(
        None,
        vec![format!(
            "The minikube tunnel has been started and bound to {}",
            get_bind_address()
        )],
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

pub fn build_set_bind_address_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Set the minikube tunnel bind address"),
        Box::new(set_bind_address_guided),
        false,
    ))
}

pub fn set_bind_address(bind_address: String) -> CommandExecutionResult {
    if get_bind_address() == bind_address {
        return Ok(PrintableResults(None, Vec::new()));
    }

    unsafe {
        BIND_ADDRESS.replace(bind_address);
    }

    let mut results: Vec<String> = Vec::new();

    results.push(format!(
        "The minikube tunnel bind address has been set to {}",
        unsafe { BIND_ADDRESS.as_ref().unwrap() }
    ));

    if let Ok(true) = check_minikube_tunnel() {
        let _ = merge_if_ok(&mut results, create_minikube_tunnel);
    }

    Ok(PrintableResults(None, results))
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
                    &["tunnel", "-c", "--bind-address", &get_bind_address()],
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
            vec![format!(
                "The minikube tunnel has been started and bound to {}",
                get_bind_address()
            )],
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

fn set_bind_address_guided() -> CommandExecutionResult {
    let bind_address = parse_string(
        "Bind address: ",
        None,
        Some(String::from(
            "No address provided as the minikube tunnel default bind address",
        )),
    )?;

    set_bind_address(bind_address)
}

fn get_bind_address() -> String {
    String::from(unsafe { BIND_ADDRESS.as_ref().unwrap_or(&String::from("127.0.0.1")) })
}
