use std::{default::default, io::Read, todo};

use crate::{start_process, CommandExecutionResult, CommandResultType::*, OptionFunc};

pub fn build_fetch_load_balancers_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("List load balancers"),
        Box::new(fetch_load_balancers),
    ))
}

pub fn fetch_load_balancers() -> CommandExecutionResult {
    let (child, _) = start_process(
        "kubectl",
        &[
            "get",
            "svc",
            "-A",
            "-l",
            "reason=kube-minion",
            "--no-headers",
        ],
        Some(String::from("Failed to fetch load balancers")),
        default(),
    )?
    .child_process()
    .take()
    .unwrap();

    let mut child = child.lock().unwrap();

    child.wait().unwrap();

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();

    Ok(PrintableResults(
        stdout
            .lines()
            .filter(|x| !x.contains("kubernetes-dashboard-lb"))
            .map(String::from)
            .collect::<Vec<_>>(),
    ))
}

pub fn create_load_balancer(
    namespace: &str,
    target: &str,
    name: &str,
    port: u16,
    target_port: u16,
) {
    todo!()
}

fn parse_load_balancer(spec: &str) -> (String, String) {
    todo!()
}
