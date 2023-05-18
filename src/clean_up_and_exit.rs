use crate::dashboard::delete_kubernetes_dashboard_load_balancer;
use crate::load_balancer::delete_all_load_balancers;
use crate::minikube_mount::delete_all_minikube_mounts;
use crate::minikube_tunnel::stop_minikube_tunnel;
use crate::socat_tunnel::delete_all_socat_tunnels;
use crate::CommandResultType::*;
use crate::{CommandExecutionResult, OptionFunc};

pub fn build_clean_up_and_exit_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Clean up and exit"),
        Box::new(clean_up_and_exit),
    ))
}

fn clean_up_and_exit() -> CommandExecutionResult {
    delete_all_load_balancers()
        .and_then(|result| match result {
            PrintableResults(_, mut results) => match delete_all_socat_tunnels() {
                Ok(PrintableResults(_, mut new_results)) => {
                    results.append(&mut new_results);
                    Ok(PrintableResults(None, results))
                }
                Ok(ChildProcess(_)) => unreachable!(),
                Err(error) => Err(error),
            },
            ChildProcess(_) => unreachable!(),
        })
        .and_then(|result| match result {
            PrintableResults(_, mut results) => match delete_all_minikube_mounts() {
                Ok(PrintableResults(_, mut new_results)) => {
                    results.append(&mut new_results);
                    Ok(PrintableResults(None, results))
                }
                Ok(ChildProcess(_)) => unreachable!(),
                Err(error) => Err(error),
            },
            ChildProcess(_) => unreachable!(),
        })
        .and_then(|result| match result {
            PrintableResults(_, results) => {
                delete_kubernetes_dashboard_load_balancer()?;
                Ok(PrintableResults(None, results))
            }
            ChildProcess(_) => unreachable!(),
        })
        .and_then(|result| match result {
            PrintableResults(_, results) => {
                stop_minikube_tunnel()?;
                Ok(PrintableResults(None, results))
            }
            ChildProcess(_) => unreachable!(),
        })
}
