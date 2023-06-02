#![feature(default_free_fn)]
#![feature(let_chains)]
#![feature(if_let_guard)]

mod clean_up_and_exit;
mod dashboard;
mod init_file;
mod load_balancer;
mod minikube_mount;
mod minikube_tunnel;
mod socat_tunnel;

use std::io::{stderr, stdin, stdout, BufRead, Write};
use std::str::FromStr;
use std::{
    io::Read,
    process::{self, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

pub use crate::clean_up_and_exit::clean_up;
pub use dashboard::{create_kubernetes_dashboard_load_balancer, set_dashboard_port};
pub use init_file::run_init_file;
pub use minikube_tunnel::create_minikube_tunnel;

use crate::clean_up_and_exit::build_clean_up_and_exit_option;
use dashboard::*;
use load_balancer::*;
use minikube_mount::*;
use minikube_tunnel::*;
use socat_tunnel::*;
use CommandResultType::*;

type CommandExecutionResult = Result<CommandResultType, String>;

pub type OptionFunc = Box<dyn Fn() -> CommandExecutionResult>;

pub enum CommandResultType {
    ChildProcess(Option<(Arc<Mutex<process::Child>>, ExitStatus)>),
    PrintableResults(Option<String>, Vec<String>),
}

impl CommandResultType {
    fn child_process(self) -> Option<(Arc<Mutex<process::Child>>, ExitStatus)> {
        match self {
            ChildProcess(maybe_child) => maybe_child,
            PrintableResults(..) => {
                panic!("CommandResultType.child_process called on a non ChildProcess variant")
            }
        }
    }
}

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

    Command::new("ssh")
        .arg("-V")
        .output()
        .map_err(|_| could_not_find("ssh"))?;

    Ok(())
}

pub fn build_options() -> Result<Vec<(String, OptionFunc, bool)>, String> {
    let do_nothing: fn() -> OptionFunc = || Box::new(|| Ok(PrintableResults(None, Vec::new())));

    Ok(vec![
        (String::from("# Dashboard"), do_nothing(), false),
        build_kubernetes_dashboard_option()?,
        (String::from("# Minikube tunnel"), do_nothing(), false),
        build_minikube_tunnel_option()?,
        (String::from("# Load balancers"), do_nothing(), false),
        build_create_load_balancer_option()?,
        build_fetch_load_balancers_option()?,
        build_delete_load_balancer_option()?,
        build_delete_all_load_balancers_option()?,
        (String::from("# Socat tunnels"), do_nothing(), false),
        build_create_socat_tunnel_option()?,
        build_fetch_socat_tunnels_option()?,
        build_delete_socat_tunnel_option()?,
        build_delete_all_socat_tunnels_option()?,
        build_set_default_connect_host_option()?,
        (String::from("# Minikube mounts"), do_nothing(), false),
        build_create_minikube_mount_option()?,
        build_fetch_minikube_mounts_option()?,
        build_delete_minikube_mount_option()?,
        build_delete_all_minikube_mounts_option()?,
        (String::from("# Clean up and exit"), do_nothing(), false),
        build_clean_up_and_exit_option()?,
        (String::from("Exit without cleaning up"), do_nothing(), true),
    ])
}

pub fn get_sys_info() -> System {
    System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::everything()))
}

pub fn print_results(result: CommandExecutionResult, stdout: bool, stderr: bool) {
    match result {
        Ok(ChildProcess(_)) => {
            let (_, result_stdout, result_stderr) = process_exited_with_success(result);

            if stdout && let Some(result_stdout) = result_stdout {
                println!("{}", result_stdout.trim());
            }
            if stderr && let Some(result_stderr) = result_stderr {
                eprintln!("{}", result_stderr.trim());
            }
        }
        Ok(PrintableResults(title, result)) => {
            let printable_results: Vec<(usize, &String)> = result.iter().enumerate().collect();
            let mut indentation = "";
            let mut print_indexes = false;

            if let Some(title) = title {
                println!("{title}");

                indentation = "\t";
                print_indexes = true;
            }

            printable_results.iter().for_each(|(i, x)| {
                let index = if print_indexes {
                    format!("{}. ", i + 1)
                } else {
                    String::new()
                };

                println!("{indentation}{index}{x}");
            });
        }
        Err(error) => {
            if stderr {
                eprintln!("{error}");
            }
        }
    }

    flush_output();
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

fn kill_process(name: &str, patterns: Vec<&str>) -> CommandExecutionResult {
    let sys_info = get_sys_info();

    let killed_processes: Vec<(&sysinfo::Process, bool)> = sys_info
        .processes_by_name(name)
        .filter(|x| {
            let cmd = x.cmd().join(" ");

            patterns
                .iter()
                .all(|pattern| cmd.contains(&String::from(*pattern)))
        })
        .map(|x| (x, x.kill_with(sysinfo::Signal::Interrupt).unwrap()))
        .collect();

    killed_processes
        .iter()
        .for_each(|(process, _)| process.wait());

    if killed_processes
        .iter()
        .any(|(_, successfully_killed)| !*successfully_killed)
    {
        Err(format!("Failed to kill {name} with patterns {patterns:?}"))
    } else {
        Ok(PrintableResults(None, Vec::new()))
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

        Err(String::from("An empty value is not allowed"))
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

        Err(String::from("An empty value is not allowed"))
    } else {
        input
            .parse::<T>()
            .map_err(|_| format!("Failed to parse {input} as a number"))
    }
}

/// It takes a closure that is expected to execute a command and return its result.
/// If the result is Ok, it appends the command's results to the `results` vector argument.
fn merge_if_ok<T>(results: &mut Vec<String>, f: T) -> Result<(), String>
where
    T: FnOnce() -> CommandExecutionResult,
{
    match f() {
        Ok(ChildProcess(Some((child, _)))) => {
            let mut child = child.lock().unwrap();
            let mut output = String::new();

            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(&mut output)
                .unwrap();

            let output = output.trim();

            if !output.is_empty() {
                results.push(output.to_string());
            }

            let mut output = String::new();

            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut output)
                .unwrap();

            let output = output.trim();

            if !output.is_empty() {
                results.push(output.to_string());
            }

            Ok(())
        }
        Ok(PrintableResults(_, mut new_results)) => {
            results.append(&mut new_results);
            Ok(())
        }
        Err(error) => Err(error),
        _ => Ok(()),
    }
}

fn flush_output() {
    stdout().lock().flush().unwrap();
    stderr().lock().flush().unwrap();
}
