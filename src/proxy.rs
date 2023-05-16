use std::io::Read;
use std::io::{stdin, stdout, BufRead, Write};
use std::str::FromStr;

use crate::{start_and_wait_process, CommandExecutionResult, CommandResultType::*, OptionFunc};

use regex::Regex;

pub fn build_fetch_load_balancers_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("List load balancers"),
        Box::new(fetch_load_balancers),
    ))
}

pub fn build_create_load_balancer_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Create load balancer"),
        Box::new(create_load_balancer_guided),
    ))
}

pub fn build_delete_load_balancer_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Delete load balancer"),
        Box::new(delete_load_balancer_guided),
    ))
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

    Ok(PrintableResults(
        stdout
            .lines()
            .filter(|x| !x.contains("kubernetes-dashboard-lb"))
            .map(String::from)
            .collect::<Vec<_>>(),
    ))
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

fn create_load_balancer(
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
        PrintableResults(results) => results,
    };

    if index >= load_balancers.len() {
        return Err(format!(
            "Index {index} does not correspond to a load balancer"
        ));
    }

    let (namespace, name) = parse_load_balancer(&load_balancers[index]);

    start_and_wait_process("kubectl", &["-n", &namespace, "delete", "svc", &name], None)
}

fn parse_string(
    prompt: &str,
    default_value: Option<String>,
    error_when_empty: Option<String>,
) -> Result<String, String> {
    let mut input = String::new();
    let mut stdin = stdin().lock();
    let mut stdout = stdout().lock();

    print!("{prompt}");
    stdout.flush().unwrap();

    input.clear();
    stdin.read_line(&mut input).unwrap();

    let input = input.trim();

    if input.is_empty() {
        if let Some(error_when_empty) = error_when_empty {
            return Err(error_when_empty);
        }

        if let Some(default_value) = default_value {
            return Ok(default_value);
        }

        return Err(String::from("An empty value is not allowed"));
    } else {
        Ok(String::from(input))
    }
}

fn parse_num<T: FromStr>(
    prompt: &str,
    default_value: Option<T>,
    error_when_empty: Option<String>,
) -> Result<T, String> {
    let mut input = String::new();
    let mut stdin = stdin().lock();
    let mut stdout = stdout().lock();

    print!("{prompt}");
    stdout.flush().unwrap();

    input.clear();
    stdin.read_line(&mut input).unwrap();

    let input = input.trim();

    if input.is_empty() {
        if let Some(error_when_empty) = error_when_empty {
            return Err(error_when_empty);
        }

        if let Some(default_value) = default_value {
            return Ok(default_value);
        }

        return Err(String::from("An empty value is not allowed"));
    } else {
        input
            .parse::<T>()
            .map_err(|_| format!("Failed to parse {input} as a number"))
    }
}

fn parse_load_balancer(spec: &str) -> (String, String) {
    let re = Regex::new(r"\s+").unwrap();
    let parts: Vec<&str> = re.split(spec).collect();

    (String::from(parts[0]), String::from(parts[1]))
}
