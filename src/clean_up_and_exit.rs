use crate::dashboard::delete_kubernetes_dashboard_load_balancer;
use crate::load_balancer::delete_all_load_balancers;
use crate::minikube_mount::delete_all_minikube_mounts;
use crate::minikube_tunnel::stop_minikube_tunnel;
use crate::socat_tunnel::delete_all_socat_tunnels;
use crate::CommandResultType::*;
use crate::{merge_if_ok, CommandExecutionResult, OptionFunc};

pub fn build_clean_up_and_exit_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Clean up and exit"),
        Box::new(clean_up_and_exit),
    ))
}

pub fn clean_up_and_exit() -> CommandExecutionResult {
    let mut results: Vec<String> = Vec::new();

    merge_if_ok(&mut results, delete_all_load_balancers)?;
    merge_if_ok(&mut results, delete_all_socat_tunnels)?;
    merge_if_ok(&mut results, delete_all_minikube_mounts)?;
    delete_kubernetes_dashboard_load_balancer()?;
    stop_minikube_tunnel()?;

    Ok(PrintableResults(None, results))
}
