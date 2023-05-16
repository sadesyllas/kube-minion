use std::{thread, time::Duration};

use sysinfo::{ProcessExt, SystemExt};

use crate::{
    get_sys_info, kill_process, start_and_wait_process, CommandExecutionResult,
    CommandResultType::*, OptionFunc,
};

pub fn create_minikube_tunnel() -> Result<(), String> {
    if let Ok(false) = check_minikube_tunnel() {
        let result = toggle_minikube_tunnel(false);

        return match result {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        };
    }

    println!("The minikube tunnel has been started");

    Ok(())
}

pub fn build_minikube_tunnel_option() -> Result<(String, OptionFunc), String> {
    let check_minikube_tunnel_result = check_minikube_tunnel();

    match check_minikube_tunnel_result {
        Ok(running) => {
            let next_state = if running { "Stop" } else { "Start" };

            Ok((
                format!("{next_state} minikube tunnel"),
                Box::new(move || toggle_minikube_tunnel(running)),
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
        let result = kill_process("minikube", "tunnel");

        println!("The minikube tunnel has been stopped");

        result
    } else {
        thread::spawn(move || {
            let _ = start_and_wait_process(
                "minikube",
                &["tunnel", "-c", "--bind-address=127.0.0.1"],
                Some(String::from("Could not start minikube tunnel")),
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

        println!("The minikube tunnel has been started");

        Ok(PrintableResults(Vec::new()))
    }
}
