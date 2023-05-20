use crate::dashboard::delete_kubernetes_dashboard_load_balancer;
use crate::load_balancer::delete_all_load_balancers;
use crate::minikube_mount::delete_all_minikube_mounts;
use crate::minikube_tunnel::stop_minikube_tunnel;
use crate::socat_tunnel::delete_all_socat_tunnels;
use crate::CommandResultType::*;
use crate::{merge_if_ok, CommandExecutionResult, OptionFunc};

pub fn build_clean_up_and_exit_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((String::from("Clean up and exit"), Box::new(clean_up), true))
}

pub fn clean_up() -> CommandExecutionResult {
    let mut results: Vec<String> = Vec::new();

    let _ = merge_if_ok(&mut results, delete_all_load_balancers);
    let _ = merge_if_ok(&mut results, delete_all_socat_tunnels);
    let _ = merge_if_ok(&mut results, delete_all_minikube_mounts);
    let _ = merge_if_ok(&mut results, delete_kubernetes_dashboard_load_balancer);
    let _ = merge_if_ok(&mut results, stop_minikube_tunnel);

    Ok(PrintableResults(None, results))
}
