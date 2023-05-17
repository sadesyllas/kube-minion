use regex::Regex;
use std::{thread, time::Duration};

use sysinfo::{ProcessExt, SystemExt};

use crate::{
    get_sys_info, parse_num, parse_string, start_and_wait_process, CommandExecutionResult,
    CommandResultType::*, OptionFunc,
};

pub fn build_fetch_socat_tunnels_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("List socat tunnels"),
        Box::new(fetch_socat_tunnels),
    ))
}

pub fn build_create_socat_tunnel_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Create socat tunnel"),
        Box::new(create_socat_tunnel_guided),
    ))
}

pub fn build_delete_socat_tunnel_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Delete socat tunnel"),
        Box::new(delete_socat_tunnel_guided),
    ))
}

pub fn build_delete_all_socat_tunnels_option() -> Result<(String, OptionFunc), String> {
    Ok((
        String::from("Delete all socat tunnels"),
        Box::new(delete_all_socat_tunnels),
    ))
}

fn fetch_socat_tunnels() -> CommandExecutionResult {
    let sys_info = get_sys_info();
    let socat_tunnels: Vec<String> = sys_info
        .processes_by_name("socat")
        .map(|x| x.cmd().join(" "))
        .filter(|x| x.contains("-lpkube-minion-socat"))
        .collect();

    let title = if socat_tunnels.is_empty() {
        None
    } else {
        Some(String::from("Socat tunnels:"))
    };

    Ok(PrintableResults(title, socat_tunnels))
}

fn create_socat_tunnel_guided() -> CommandExecutionResult {
    let protocol = parse_string(
        "Protocol (either tcp or udp / leave empty for tcp): ",
        Some(String::from("tcp")),
        None,
    )?;

    let listening_port: u16 = parse_num(
        "Listening port: ",
        None,
        Some(format!(
            "A listening port is required to create a {protocol} socat tunnel"
        )),
    )?;

    let connect_host = parse_string(
        "Connect host (leave empty for localhost): ",
        Some(String::from("localhost")),
        None,
    )?;

    let connect_port: u16 = parse_num(
        "Connect port: ",
        None,
        Some(format!(
            "A connect port is required to create a {protocol} socat tunnel"
        )),
    )?;

    create_socat_tunnel(&protocol, listening_port, &connect_host, connect_port)
}

fn check_socat_tunnel(
    sys_info: &sysinfo::System,
    listening_port: u16,
    connect_host: &str,
    connect_port: u16,
) -> Option<sysinfo::Pid> {
    sys_info
        .processes_by_name("socat")
        .find(|x| {
            let cmd = x.cmd().join(" ");

            cmd.contains("-lpkube-minion-socat")
                && cmd.contains(&format!("-listen:{listening_port}"))
                && cmd.contains(&format!(":{connect_host}:{connect_port}"))
        })
        .map(|x| x.pid())
}

fn create_socat_tunnel(
    protocol: &str,
    listening_port: u16,
    connect_host: &str,
    connect_port: u16,
) -> CommandExecutionResult {
    {
        let protocol = String::from(protocol);
        let connect_host = String::from(connect_host);
        thread::spawn(move || {
            let _ = start_and_wait_process(
                "socat",
                &[
                    "-lpkube-minion-socat",
                    &format!("{protocol}-listen:{listening_port},fork,reuseaddr"),
                    &format!("{protocol}:{connect_host}:{connect_port}"),
                ],
                Some(String::from("Could not start socat tunnel")),
            );
        });
    }

    {
        let sys_info = get_sys_info();
        let mut cnt = 0;

        while let None = check_socat_tunnel(&sys_info, listening_port, connect_host, connect_port) && cnt < 5 {
            cnt += 1;
            thread::sleep(Duration::from_secs(1));
        }

        if cnt == 5 {
            return Err(String::from(
                "Failed to verify if socat tunnel has been started",
            ));
        }
    }

    println!(
        "Started socat tunnel listening on port {listening_port} \
        and connecting to {connect_host}:{connect_port}"
    );

    Ok(PrintableResults(None, Vec::new()))
}

fn delete_socat_tunnel_guided() -> CommandExecutionResult {
    let index: usize = parse_num(
        "Index: ",
        None,
        Some(format!("An index is required to delete a socat tunnel")),
    )?;

    delete_socat_tunnel(index - 1)
}

fn delete_socat_tunnel(index: usize) -> CommandExecutionResult {
    let socat_tunnels = match fetch_socat_tunnels()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    if index >= socat_tunnels.len() {
        return Err(format!(
            "Index {index} does not correspond to a socat tunnel"
        ));
    }

    let (listening_port, connect_host, connect_port) = parse_socat_tunnel(&socat_tunnels[index]);

    let sys_info = get_sys_info();

    let mut results: Vec<String> = Vec::new();

    if let Some(pid) = check_socat_tunnel(&sys_info, listening_port, &connect_host, connect_port) &&
        let Some(process) = get_sys_info().process(pid) &&
        process.kill() {
        results.push(format!(
            "Stopped socat tunnel listening on port {listening_port} \
            and connecting to {connect_host}:{connect_port}", ));
    }

    // start_and_wait_process("kubectl", &["-n", &namespace, "delete", "svc", &name], None)
    Ok(PrintableResults(None, results))
}

fn delete_all_socat_tunnels() -> CommandExecutionResult {
    let socat_tunnels = match fetch_socat_tunnels()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    let sys_info = get_sys_info();

    let mut results: Vec<String> = Vec::new();

    for socat_tunnel in &socat_tunnels {
        let (listening_port, connect_host, connect_port) = parse_socat_tunnel(socat_tunnel);

        if let Some(pid) = check_socat_tunnel(&sys_info, listening_port, &connect_host, connect_port) &&
            let Some(process) = get_sys_info().process(pid) {
            if process.kill() {
                results.push(format!(
                    "Stopped socat tunnel listening on port {listening_port} \
                    and connecting to {connect_host}:{connect_port}", ));
            } else {
                results.push(format!(
                    "Failed to stop socat tunnel listening on port {listening_port} \
                    and connecting to {connect_host}:{connect_port}", ));
            }
        }
    }

    Ok(PrintableResults(None, results))
}

fn parse_socat_tunnel(spec: &str) -> (u16, String, u16) {
    let re = Regex::new(
        r".+?-listen:(?<listening_port>[0-9]+).+?:(?<connect_host>.+):(?<connect_port>[0-9]+)$",
    )
    .unwrap();

    let captures = re.captures(spec).unwrap();

    let listening_port: u16 = captures
        .name("listening_port")
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    let connect_host: String = captures.name("connect_host").unwrap().as_str().to_string();
    let connect_port: u16 = captures
        .name("connect_port")
        .unwrap()
        .as_str()
        .parse()
        .unwrap();

    (listening_port, connect_host, connect_port)
}
