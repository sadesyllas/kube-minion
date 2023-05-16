#![feature(default_free_fn)]

mod dashboard;
mod proxy;
mod tunnel;

use std::{
    io::Read,
    process::{self, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

use dashboard::build_kubernetes_dashboard_option;
use proxy::build_fetch_load_balancers_option;
use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

type CommandExecutionResult = Result<CommandResultType, String>;

type OptionFunc = Box<dyn Fn() -> CommandExecutionResult>;

pub use dashboard::create_kubernetes_dashboard_load_balancer;
pub use tunnel::create_minikube_tunnel;

pub enum CommandResultType {
    ChildProcess(Option<(Arc<Mutex<process::Child>>, ExitStatus)>),
    PrintableResults(Vec<String>),
}

impl CommandResultType {
    fn child_process(self) -> Option<(Arc<Mutex<process::Child>>, ExitStatus)> {
        match self {
            ChildProcess(maybe_child) => maybe_child,
            PrintableResults(_) => {
                panic!("CommandResultType.child_process called on a non ChildProcess variant")
            }
        }
    }
}

use crate::proxy::build_create_load_balancer_option;
use tunnel::build_minikube_tunnel_option;
use CommandResultType::*;

pub fn verify_dependencies() -> Result<(), String> {
    let could_not_find = |what| format!("Could not find {what} in path");

    Command::new("minikube")
        .arg("version")
        .output()
        .map_err(|_| could_not_find("minikube"))?;

    Command::new("kubectl")
        .arg("--version")
        .output()
        .map_err(|_| could_not_find("kubectl"))?;

    Command::new("socat")
        .arg("-V")
        .output()
        .map_err(|_| could_not_find("socat"))?;

    Ok(())
}

pub fn build_options() -> Result<Vec<(String, OptionFunc)>, String> {
    Ok(vec![
        build_kubernetes_dashboard_option()?,
        build_minikube_tunnel_option()?,
        build_fetch_load_balancers_option()?,
        build_create_load_balancer_option()?,
    ])
}

pub fn get_sys_info() -> System {
    System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::everything()))
}

fn start_and_wait_process(
    command: &str,
    args: &[&str],
    custom_error: Option<String>,
) -> CommandExecutionResult {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|x| {
            if let Some(y) = custom_error {
                y
            } else {
                x.to_string()
            }
        })?;

    match child.wait().map_err(|x| x.to_string()) {
        Ok(exit_status) => Ok(ChildProcess(Some((
            Arc::new(Mutex::new(child)),
            exit_status,
        )))),
        Err(error) => Err(error),
    }
}

fn kill_process(name: &str, pattern: &str) -> CommandExecutionResult {
    let sys_info = get_sys_info();

    let killed_processes: Vec<(&sysinfo::Process, bool)> = sys_info
        .processes_by_name(name)
        .filter(|x| x.cmd().join(" ").contains(&String::from(pattern)))
        .map(|x| (x, x.kill_with(sysinfo::Signal::Interrupt).unwrap()))
        .collect();

    killed_processes
        .iter()
        .for_each(|(process, _)| process.wait());

    if killed_processes
        .iter()
        .any(|(_, successfully_killed)| !*successfully_killed)
    {
        Err(format!("Failed to kill {name} with pattern {pattern}"))
    } else {
        Ok(ChildProcess(None))
    }
}

fn process_exited_with_success(
    child_process_result: CommandExecutionResult,
) -> (bool, Option<String>, Option<String>) {
    match child_process_result {
        Ok(ChildProcess(Some((child, exit_status)))) => {
            let mut child = child.lock().unwrap();

            let mut stdout = String::new();
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(&mut stdout)
                .unwrap();
            let stdout = if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            };

            let mut stderr = String::new();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            let stderr = if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            };

            (exit_status.success(), stdout, stderr)
        }
        Ok(_) => (true, None, None),
        Err(error) => (false, None, Some(error)),
    }
}
