use crate::{
    process_exited_with_success, start_and_wait_process, CommandExecutionResult,
    CommandResultType::*, OptionFunc,
};

pub fn create_kubernetes_dashboard_load_balancer() -> Result<(), String> {
    if let Ok(false) = check_kubernetes_dashboard() {
        let result = toggle_kubernetes_dashboard_load_balancer(false);

        return match result {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        };
    }

    println!("The kubernetes dashboard load balancer can be accessed at http://127.0.0.1:51515");

    Ok(())
}

pub fn delete_kubernetes_dashboard_load_balancer() -> CommandExecutionResult {
    if let Ok(true) = check_kubernetes_dashboard() {
        toggle_kubernetes_dashboard_load_balancer(true)
    } else {
        Ok(PrintableResults(None, Vec::new()))
    }
}

pub fn build_kubernetes_dashboard_option() -> Result<(String, OptionFunc), String> {
    let check_kubernetes_dashboard_result = check_kubernetes_dashboard();

    match check_kubernetes_dashboard_result {
        Ok(running) => {
            let next_state = if running { "Delete" } else { "Create" };

            Ok((
                format!("{next_state} kubernetes dashboard load balancer"),
                Box::new(move || toggle_kubernetes_dashboard_load_balancer(running)),
            ))
        }
        Err(error) => Err(format!(
            "Error in build_kubernetes_dashboard_option: {error}"
        )),
    }
}

fn check_kubernetes_dashboard() -> Result<bool, String> {
    let result = start_and_wait_process(
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

fn toggle_kubernetes_dashboard_load_balancer(running: bool) -> CommandExecutionResult {
    if running {
        let result = start_and_wait_process(
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
        );

        match process_exited_with_success(result) {
            (true, _, _) => {
                println!("The kubernetes dashboard load balancer has been deleted");

                Ok(PrintableResults(None, Vec::new()))
            }
            (false, _, Some(error)) => Err(error),
            (false, _, _) => Err(String::from(
                "Failed to delete the kubernetes dashboard load balancer",
            )),
        }
    } else {
        let result = start_and_wait_process(
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
        );

        match process_exited_with_success(result) {
            (true, _, _) => {
                println!("The kubernetes dashboard load balancer can be accessed at http://127.0.0.1:51515");

                Ok(PrintableResults(None, Vec::new()))
            }
            (false, _, Some(error)) => Err(error),
            (false, _, _) => Err(String::from(
                "Failed to create the kubernetes dashboard load balancer",
            )),
        }
    }
}
