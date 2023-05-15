#![feature(default_free_fn)]

mod dashboard;
mod proxy;
mod tunnel;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io::Read,
    process::{self, Command, Stdio},
    sync::{Arc, Mutex, MutexGuard},
};

use dashboard::build_kubernetes_dashboard_option;
use proxy::build_fetch_load_balancers_option;
use sysinfo::{ProcessExt, System, SystemExt};

pub type AlreadyStartedCommands = Arc<Mutex<HashMap<u64, Arc<Mutex<process::Child>>>>>;

type CommandExecutionResult = Result<CommandResultType, String>;

type OptionFunc = Box<dyn Fn() -> CommandExecutionResult>;

pub use dashboard::create_kubernetes_dashboard_load_balancer;
pub use proxy::{create_load_balancer, fetch_load_balancers};
pub use tunnel::create_minikube_tunnel;

pub enum CommandResultType {
    ChildProcess(Option<(Arc<Mutex<process::Child>>, u64)>),
    PrintableResults(Vec<String>),
}

impl CommandResultType {
    fn child_process(self) -> Option<(Arc<Mutex<process::Child>>, u64)> {
        match self {
            ChildProcess(maybe_child) => maybe_child,
            PrintableResults(_) => {
                panic!("CommandResultType.child_process called on a non ChildProcess variant")
            }
        }
    }
}

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

pub fn build_options(
    already_started_commands: AlreadyStartedCommands,
    sysinfo: Arc<Mutex<System>>,
) -> Result<Vec<(String, OptionFunc)>, String> {
    Ok(vec![
        build_kubernetes_dashboard_option(Arc::clone(&already_started_commands))?,
        build_minikube_tunnel_option(Arc::clone(&already_started_commands), Arc::clone(&sysinfo))?,
        build_fetch_load_balancers_option()?,
    ])
}

pub fn refresh_sysinfo(sysinfo: &Arc<Mutex<System>>) -> MutexGuard<System> {
    let mut sysinfo = sysinfo.lock().unwrap();
    sysinfo.refresh_processes();
    sysinfo
}

fn start_process(
    command: &str,
    args: &[&str],
    custom_error: Option<String>,
    already_started_commands: AlreadyStartedCommands,
) -> CommandExecutionResult {
    let command_hash = {
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);
        args.iter().for_each(|x| x.hash(&mut hasher));
        hasher.finish()
    };

    let already_started_commands = already_started_commands.lock().unwrap();
    let already_started_child = already_started_commands.get(&command_hash);

    if let Some(entry) = already_started_child {
        return Ok(ChildProcess(Some((Arc::clone(entry), command_hash))));
    }

    let child = Command::new(command)
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

    Ok(ChildProcess(Some((
        Arc::new(Mutex::new(child)),
        command_hash,
    ))))
}

fn kill_process(name: &str, pattern: &str, sysinfo: Arc<Mutex<System>>) -> CommandExecutionResult {
    let sysinfo = refresh_sysinfo(&sysinfo);

    let killed_processes: Vec<(&sysinfo::Process, bool)> = sysinfo
        .processes_by_name(name)
        .filter(|x| x.cmd().join(" ").contains(&String::from(pattern)))
        .map(|x| (x, x.kill()))
        .collect();

    killed_processes
        .iter()
        .for_each(|(process, _)| process.wait());

    if killed_processes
        .iter()
        .any(|(_, successfully_killed)| !successfully_killed)
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
        Ok(ChildProcess(Some((child, _)))) => {
            let mut child = child.lock().unwrap();
            let exit_status = child.wait().unwrap();

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
