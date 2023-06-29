use regex::Regex;
use std::{fs, thread, time::Duration};

use sysinfo::{ProcessExt, SystemExt};

use crate::{
    get_sys_info, merge_if_ok, parse_num, parse_string, print_results, start_and_wait_process,
    CommandExecutionResult, CommandResultType::*, OptionFunc,
};

pub fn build_create_minikube_mount_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Create minikube mount"),
        Box::new(create_minikube_mount_guided),
        false,
    ))
}

pub fn build_fetch_minikube_mounts_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("List minikube mounts"),
        Box::new(fetch_minikube_mounts),
        false,
    ))
}

pub fn build_delete_minikube_mount_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Delete minikube mount"),
        Box::new(delete_minikube_mount_guided),
        false,
    ))
}

pub fn build_delete_all_minikube_mounts_option() -> Result<(String, OptionFunc, bool), String> {
    Ok((
        String::from("Delete all minikube mounts"),
        Box::new(delete_all_minikube_mounts),
        false,
    ))
}

pub fn create_minikube_mount(host_path: &str, minikube_path: &str) -> CommandExecutionResult {
    if !fs::metadata(host_path).map_err(|x| x.to_string())?.is_dir() {
        return Err(format!("{host_path} is not a valid host directory path"));
    }

    if check_minikube_mount(host_path, minikube_path).is_some() {
        return Ok(PrintableResults(None, vec![
            format!("Minikube mount from host path {host_path} to minikube path {minikube_path} already exists")
        ]));
    }

    {
        let host_path = String::from(host_path);
        let minikube_path = String::from(minikube_path);
        thread::spawn(move || {
            print_results(
                start_and_wait_process(
                    "minikube",
                    &["mount", &format!("{host_path}:{minikube_path}")],
                    Some(format!(
                    "Failed to stop minikube mount from host path {host_path} to minikube path \
                    {minikube_path}"
                )),
                ),
                false,
                true,
            );
        });
    }

    {
        let mut cnt = 0;

        while let None = check_minikube_mount(host_path, minikube_path) && cnt < 5 {
            cnt += 1;
            thread::sleep(Duration::from_secs(1));
        }

        if cnt == 5 {
            return Err(String::from(
                "Failed to verify if minikube mount has been created",
            ));
        }
    }

    Ok(PrintableResults(
        None,
        vec![format!(
            "Created minikube mount from host path {host_path} to minikube path {minikube_path}"
        )],
    ))
}

pub fn delete_minikube_mount(host_path: &str, minikube_path: &str) -> CommandExecutionResult {
    let mut results: Vec<String> = Vec::new();

    if let Some(pid) = check_minikube_mount(host_path, minikube_path) &&
            let Some(process) = get_sys_info().process(pid) {
            if process.kill_with(sysinfo::Signal::Interrupt).is_some() {
                results.push(format!("Stopped minikube mount from host path {host_path} to minikube path {minikube_path}"));
            } else {
                results.push(format!("Failed to stop minikube mount from host path {host_path} to minikube path {minikube_path}"));
            }
        }

    Ok(PrintableResults(None, results))
}

pub fn delete_all_minikube_mounts() -> CommandExecutionResult {
    let minikube_mounts = match fetch_minikube_mounts()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    let mut results: Vec<String> = Vec::new();

    for minikube_mount in &minikube_mounts {
        let (host_path, minikube_path) = parse_minikube_mount(minikube_mount);

        merge_if_ok(&mut results, || {
            delete_minikube_mount(&host_path, &minikube_path)
        })?;
    }

    Ok(PrintableResults(None, results))
}

fn fetch_minikube_mounts() -> CommandExecutionResult {
    let sys_info = get_sys_info();
    let minikube_mounts: Vec<String> = sys_info
        .processes_by_name("minikube")
        .map(|x| x.cmd().join(" "))
        .filter(|x| x.contains("mount"))
        .collect();

    let title = if minikube_mounts.is_empty() {
        None
    } else {
        Some(String::from("Minikube mounts:"))
    };

    Ok(PrintableResults(title, minikube_mounts))
}

fn create_minikube_mount_guided() -> CommandExecutionResult {
    let host_path = parse_string(
        "Host path: ",
        None,
        Some(String::from(
            "A host path is required to create a minikube mount",
        )),
    )?;

    let minikube_path = parse_string(
        "Minikube path: ",
        None,
        Some(String::from(
            "A minikube path is required to create a minikube mount",
        )),
    )?;

    create_minikube_mount(&host_path, &minikube_path)
}

fn check_minikube_mount(host_path: &str, minikube_path: &str) -> Option<sysinfo::Pid> {
    get_sys_info()
        .processes_by_name("minikube")
        .find(|x| {
            let cmd = x.cmd().join(" ");

            cmd.contains("mount") && cmd.contains(&format!("{host_path}:{minikube_path}"))
        })
        .map(|x| x.pid())
}

fn delete_minikube_mount_guided() -> CommandExecutionResult {
    let index: usize = parse_num(
        "Index: ",
        None,
        Some(String::from(
            "An index is required to delete a minikube mount",
        )),
    )?;

    delete_minikube_mount_by_index(index - 1)
}

fn delete_minikube_mount_by_index(index: usize) -> CommandExecutionResult {
    let minikube_mounts = match fetch_minikube_mounts()? {
        ChildProcess(_) => unreachable!(),
        PrintableResults(_, results) => results,
    };

    if index >= minikube_mounts.len() {
        return Err(format!(
            "Index {index} does not correspond to a minikube mount"
        ));
    }

    let (host_path, minikube_path) = parse_minikube_mount(&minikube_mounts[index]);

    let mut results: Vec<String> = Vec::new();

    if let Some(pid) = check_minikube_mount(&host_path, &minikube_path) &&
        let Some(process) = get_sys_info().process(pid) &&
        process.kill_with(sysinfo::Signal::Interrupt).is_some() {
        results.push(format!("Stopped minikube mount from host path {host_path} to minikube path {minikube_path}"));
    }

    // start_and_wait_process("kubectl", &["-n", &namespace, "delete", "svc", &name], None)
    Ok(PrintableResults(None, results))
}

#[allow(clippy::invalid_regex)] // clippy bug?
fn parse_minikube_mount(spec: &str) -> (String, String) {
    let re = Regex::new(r".+?\s+(?<host_path>[^:]+):(?<minikube_path>[^:]+)$").unwrap();
    let captures = re.captures(spec).unwrap();

    let host_path: String = captures.name("host_path").unwrap().as_str().to_string();
    let minikube_path: String = captures.name("minikube_path").unwrap().as_str().to_string();

    (host_path, minikube_path)
}
