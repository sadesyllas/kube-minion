use std::io::Read;

use crate::{
    parse_num, parse_string, start_and_wait_process, CommandExecutionResult, CommandResultType::*,
    OptionFunc,
};

use regex::Regex;

pub fn build_create_load_balancer_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Create load balancer"),
        Box::new(create_load_balancer_guided),
        false,
    ))
}

pub fn build_fetch_load_balancers_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("List load balancers"),
        Box::new(fetch_load_balancers),
        false,
    ))
}

pub fn build_delete_load_balancer_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Delete load balancer"),
        Box::new(delete_load_balancer_guided),
        false,
    ))
}

pub fn build_delete_all_load_balancers_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Delete all load balancers"),
        Box::new(delete_all_load_balancers),
        false,
    ))
}

pub fn create_load_balancer(
    namespace: &str,
    resource_type: &str,
    name: &str,
    port: u16,
    target_port: u16,
) -> CommandExecutionResult {
    start_and_wait_process(
        "kubectl",
        &[
            "-n",
            namespace,
            "expose",
            resource_type,
            name,
            "--type",
            "LoadBalancer",
            "--name",
            &(String::from(name) + "-lb"),
            "--port",
            &port.to_string(),
            "--target-port",
            &target_port.to_string(),
            "-l",
            "reason=kube-minion",
        ],
        None,
    )
}

pub fn delete_all_load_balancers() -> CommandExecutionResult {
    let load_balancers = match fetch_load_balancers()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    let mut results: Vec<String> = Vec::new();

    for load_balancer in &load_balancers {
        let (namespace, name) = parse_load_balancer(load_balancer);

        let result =
            start_and_wait_process("kubectl", &["-n", &namespace, "delete", "svc", &name], None)?;

        match result {
            ChildProcess(Some((child, exit_status))) => {
                if !exit_status.success() {
                    return Ok(ChildProcess(Some((child, exit_status))));
                }

                let mut output = String::new();

                child
                    .lock()
                    .unwrap()
                    .stdout
                    .take()
                    .unwrap()
                    .read_to_string(&mut output)
                    .unwrap();

                let output = output.trim();

                if !output.is_empty() {
                    results.push(String::from(output));
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(PrintableResults(None, results))
}

fn fetch_load_balancers() -> CommandExecutionResult {
    let (child, _) = start_and_wait_process(
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
    )?
    .child_process()
    .take()
    .unwrap();

    let mut child = child.lock().unwrap();

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();

    let load_balancers = stdout
        .lines()
        .filter(|x| !x.contains("kubernetes-dashboard-lb"))
        .map(String::from)
        .collect::<Vec<_>>();

    let title = if load_balancers.is_empty() {
        None
    } else {
        Some(String::from("Load balancers:"))
    };

    Ok(PrintableResults(title, load_balancers))
}

fn create_load_balancer_guided() -> CommandExecutionResult {
    let namespace = parse_string(
        "Namespace (leave empty for default namespace): ",
        Some(String::from("default")),
        None,
    )?;

    let resource_type = parse_string(
        "Resource type (leave empty for services): ",
        Some(String::from("svc")),
        None,
    )?;

    let name = parse_string(
        "Name: ",
        None,
        Some(format!("The name of the {resource_type} is required")),
    )?;

    let port: u16 = parse_num(
        "Port: ",
        None,
        Some(format!(
            "A port is required to create a load balancer for {resource_type}/{name}"
        )),
    )?;

    let target_port: u16 = parse_num(
        "Target port (leave empty to use the same as --port): ",
        Some(port),
        None,
    )?;

    create_load_balancer(&namespace, &resource_type, &name, port, target_port)
}

fn delete_load_balancer_guided() -> CommandExecutionResult {
    let index: usize = parse_num(
        "Index: ",
        None,
        Some(format!("An index is required to delete a load balancer")),
    )?;

    delete_load_balancer(index - 1)
}

fn delete_load_balancer(index: usize) -> CommandExecutionResult {
    let load_balancers = match fetch_load_balancers()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    if index >= load_balancers.len() {
        return Err(format!(
            "Index {index} does not correspond to a load balancer"
        ));
    }

    let (namespace, name) = parse_load_balancer(&load_balancers[index]);

    start_and_wait_process("kubectl", &["-n", &namespace, "delete", "svc", &name], None)
}

fn parse_load_balancer(spec: &str) -> (String, String) {
    let re = Regex::new(r"\s+").unwrap();
    let parts: Vec<&str> = re.split(spec).collect();

    (String::from(parts[0]), String::from(parts[1]))
}
