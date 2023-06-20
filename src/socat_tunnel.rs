use regex::Regex;
use std::{thread, time::Duration};

use sysinfo::{ProcessExt, SystemExt};

use crate::{
    get_sys_info, merge_if_ok, parse_num, parse_string, print_results, start_and_wait_process,
    CommandExecutionResult, CommandResultType::*, OptionFunc,
};

static mut DEFAULT_CONNECT_HOST: Option<String> = None;

pub fn build_create_socat_tunnel_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Create socat tunnel"),
        Box::new(create_socat_tunnel_guided),
        false,
    ))
}

pub fn build_fetch_socat_tunnels_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("List socat tunnels"),
        Box::new(fetch_socat_tunnels),
        false,
    ))
}

pub fn build_delete_socat_tunnel_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Delete socat tunnel"),
        Box::new(delete_socat_tunnel_guided),
        false,
    ))
}

pub fn build_delete_all_socat_tunnels_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Delete all socat tunnels"),
        Box::new(delete_all_socat_tunnels),
        false,
    ))
}

pub fn build_set_default_connect_host_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Set socat default connect host"),
        Box::new(set_default_connect_host_guided),
        false,
    ))
}

pub fn create_socat_tunnel(
    protocol: &str,
    listening_port: u16,
    connect_host: &str,
    connect_port: u16,
) -> CommandExecutionResult {
    {
        let protocol = String::from(protocol);
        let connect_host = String::from(connect_host);
        thread::spawn(move || {
            print_results(
                start_and_wait_process(
                    "socat",
                    &[
                        "-lpkube-minion-socat",
                        &format!("{protocol}-listen:{listening_port},fork,reuseaddr"),
                        &format!("{protocol}:{connect_host}:{connect_port}"),
                    ],
                    Some(format!(
                    "Failed to start socat tunnel listening on port {listening_port}/{protocol} \
                    and connecting to {connect_host}:{connect_port}"
                )),
                ),
                false,
                true,
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

    Ok(PrintableResults(None, vec![
        format!("Started socat tunnel listening on port {listening_port} and connecting to {connect_host}:{connect_port}")
    ]))
}

pub fn delete_socat_tunnel(
    listening_port: u16,
    connect_host: &str,
    connect_port: u16,
) -> CommandExecutionResult {
    let sys_info = get_sys_info();

    let mut results: Vec<String> = Vec::new();

    if let Some(pid) = check_socat_tunnel(&sys_info, listening_port, connect_host, connect_port) &&
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

    Ok(PrintableResults(None, results))
}

pub fn set_default_connect_host(connect_host: String) -> String {
    unsafe {
        DEFAULT_CONNECT_HOST.replace(connect_host);
    }

    format!("Socat default connect host has been set to {}", unsafe {
        DEFAULT_CONNECT_HOST.as_ref().unwrap()
    })
}

pub fn delete_all_socat_tunnels() -> CommandExecutionResult {
    let socat_tunnels = match fetch_socat_tunnels()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    let mut results: Vec<String> = Vec::new();

    for socat_tunnel in &socat_tunnels {
        let (listening_port, connect_host, connect_port) = parse_socat_tunnel(socat_tunnel);

        merge_if_ok(&mut results, || {
            delete_socat_tunnel(listening_port, &connect_host, connect_port)
        })?;
    }

    Ok(PrintableResults(None, results))
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
        Some(String::from(unsafe {
            DEFAULT_CONNECT_HOST
                .as_ref()
                .unwrap_or(&String::from("localhost"))
        })),
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

fn delete_socat_tunnel_guided() -> CommandExecutionResult {
    let index: usize = parse_num(
        "Index: ",
        None,
        Some(String::from(
            "An index is required to delete a socat tunnel",
        )),
    )?;

    delete_socat_tunnel_by_index(index - 1)
}

fn delete_socat_tunnel_by_index(index: usize) -> CommandExecutionResult {
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

#[allow(clippy::invalid_regex)] // clippy bug?
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

fn set_default_connect_host_guided() -> CommandExecutionResult {
    let connect_host = parse_string(
        "Default connect host: ",
        None,
        Some(String::from(
            "No host provided as the socat default connect host",
        )),
    )?;

    Ok(PrintableResults(
        None,
        vec![set_default_connect_host(connect_host)],
    ))
}
