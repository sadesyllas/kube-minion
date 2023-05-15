use std::sync::Arc;

use crate::{
    process_exited_with_success, start_process, AlreadyStartedCommands, CommandExecutionResult,
    CommandResultType::*, OptionFunc,
};

pub fn create_kubernetes_dashboard_load_balancer(
    already_started_commands: AlreadyStartedCommands,
) -> Result<(), String> {
    if let Ok(false) = check_kubernetes_dashboard(Arc::clone(&already_started_commands)) {
        let result =
            toggle_kubernetes_dashboard_load_balancer(false, Arc::clone(&already_started_commands));

        return match result {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        };
    }

    Ok(())
}

pub fn build_kubernetes_dashboard_option(
    already_started_commands: AlreadyStartedCommands,
) -> Result<(String, OptionFunc), String> {
    let check_kubernetes_dashboard_result =
        check_kubernetes_dashboard(Arc::clone(&already_started_commands));

    match check_kubernetes_dashboard_result {
        Ok(running) => {
            let next_state = if running { "Stop proxying" } else { "Proxy" };

            Ok((
                format!("{next_state} kubernetes dashboard"),
                Box::new(move || {
                    toggle_kubernetes_dashboard_load_balancer(
                        running,
                        Arc::clone(&already_started_commands),
                    )
                }),
            ))
        }
        Err(error) => Err(format!(
            "Error in build_kubernetes_dashboard_option: {error}"
        )),
    }
}

fn check_kubernetes_dashboard(
    already_started_commands: AlreadyStartedCommands,
) -> Result<bool, String> {
    let result = start_process(
        "kubectl",
        &[
            "-n",
            "kubernetes-dashboard",
            "get",
            "svc",
            "kubernetes-dashboard-lb",
            "--no-headers",
        ],
        None,
        Arc::clone(&already_started_commands),
    );

    match process_exited_with_success(result) {
        (true, _, _) => Ok(true),
        (false, _, Some(_)) => Ok(false),
        (false, _, _) => {
            eprintln!("Failed to check if the kubernetes dashboard load balancer exists");

            Ok(false)
        }
    }
}

fn toggle_kubernetes_dashboard_load_balancer(
    running: bool,
    already_started_commands: AlreadyStartedCommands,
) -> CommandExecutionResult {
    if running {
        let result = start_process(
            "kubectl",
            &[
                "-n",
                "kubernetes-dashboard",
                "delete",
                "svc",
                "kubernetes-dashboard-lb",
            ],
            Some(String::from(
                "Could not delete kubernetes dashboard load balancer",
            )),
            Arc::clone(&already_started_commands),
        );

        match process_exited_with_success(result) {
            (true, _, _) => {
                println!("The kubernetes dashboard load balancer has been deleted\n");

                Ok(PrintableResults(Vec::new()))
            }
            (false, _, Some(error)) => Err(error),
            (false, _, _) => Err(String::from(
                "Failed to delete the kubernetes dashboard load balancer",
            )),
        }
    } else {
        let result = start_process(
            "kubectl",
            &[
                "-n",
                "kubernetes-dashboard",
                "expose",
                "svc",
                "kubernetes-dashboard",
                "--name",
                "kubernetes-dashboard-lb",
                "--type",
                "LoadBalancer",
                "--port",
                "51515",
                "--target-port",
                "9090",
                "-l",
                "reason=kube-minion",
            ],
            Some(String::from("Could not proxy kubernetes dashboard")),
            Arc::clone(&already_started_commands),
        );

        match process_exited_with_success(result) {
            (true, _, _) => {
                println!("The kubernetes dashboard load balancer can be accessed at http://127.0.0.1:51515\n");

                Ok(PrintableResults(Vec::new()))
            }
            (false, _, Some(error)) => Err(error),
            (false, _, _) => Err(String::from(
                "Failed to create the kubernetes dashboard load balancer",
            )),
        }
    }
}
