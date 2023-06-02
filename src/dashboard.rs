use crate::{
    process_exited_with_success, start_and_wait_process, CommandExecutionResult,
    CommandResultType::*, OptionFunc,
};

static mut DASHBOARD_PORT: u16 = 51515;

pub fn set_dashboard_port(port: u16) {
    unsafe {
        DASHBOARD_PORT = port;
    }
}

pub fn create_kubernetes_dashboard_load_balancer() -> CommandExecutionResult {
    if check_kubernetes_dashboard().is_err() {
        toggle_kubernetes_dashboard_load_balancer(false)
    } else {
        Ok(PrintableResults(
            None,
            vec![format!(
                "The kubernetes dashboard load balancer can be accessed at http://127.0.0.1:{}",
                unsafe { DASHBOARD_PORT.to_string() }
            )],
        ))
    }
}

pub fn delete_kubernetes_dashboard_load_balancer() -> CommandExecutionResult {
    if check_kubernetes_dashboard().is_ok() {
        toggle_kubernetes_dashboard_load_balancer(true)
    } else {
        Ok(PrintableResults(None, Vec::new()))
    }
}

pub fn build_kubernetes_dashboard_option() -> Result<(String, OptionFunc, bool), String> {
    let check_kubernetes_dashboard_result = check_kubernetes_dashboard();

    let (running, next_option) = match check_kubernetes_dashboard_result {
        Ok(_) => (true, "Delete"),
        _ => (false, "Create"),
    };

    Ok((
        format!("{next_option} kubernetes dashboard load balancer"),
        Box::new(move || toggle_kubernetes_dashboard_load_balancer(running)),
        false,
    ))
}

fn get_dashboard_port() -> String {
    unsafe { DASHBOARD_PORT.to_string() }
}

fn check_kubernetes_dashboard() -> CommandExecutionResult {
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
        (true, _, _) => Ok(PrintableResults(None, Vec::new())),
        (false, _, Some(error)) => Err(error),
        (false, _, _) => Err(String::from(
            "Failed to check if the kubernetes dashboard load balancer exists",
        )),
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
            (true, _, _) => Ok(PrintableResults(
                None,
                vec![String::from(
                    "The kubernetes dashboard load balancer has been deleted",
                )],
            )),
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
                &get_dashboard_port(),
                "--target-port",
                "9090",
                "-l",
                "reason=kube-minion",
            ],
            Some(String::from("Could not proxy kubernetes dashboard")),
        );

        match process_exited_with_success(result) {
            (true, _, _) => Ok(PrintableResults(
                None,
                vec![format!(
                    "The kubernetes dashboard load balancer can be accessed at http://127.0.0.1:{}",
                    get_dashboard_port()
                )],
            )),
            (false, _, Some(error)) => Err(error),
            (false, _, _) => Err(String::from(
                "Failed to create the kubernetes dashboard load balancer",
            )),
        }
    }
}
