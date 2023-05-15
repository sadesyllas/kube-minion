use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
    println,
    sync::{Arc, Mutex},
};

use kube_minion::{
    self, build_options, create_kubernetes_dashboard_load_balancer, create_minikube_tunnel,
    refresh_sysinfo, verify_dependencies, CommandResultType::*,
};
use sysinfo::{Pid, PidExt, ProcessExt, System, SystemExt};

fn main() -> Result<(), String> {
    verify_dependencies()?;

    let already_started_commands: kube_minion::AlreadyStartedCommands =
        Arc::new(Mutex::new(HashMap::new()));

    let sysinfo = Arc::new(Mutex::new(System::new_all()));

    create_kubernetes_dashboard_load_balancer(Arc::clone(&already_started_commands)).unwrap();

    // create_minikube_tunnel(Arc::clone(&already_started_commands), Arc::clone(&sysinfo)).unwrap();

    loop {
        let options = build_options(Arc::clone(&already_started_commands), Arc::clone(&sysinfo))?;

        println!("0. Refresh state");

        for (index, (description, _)) in options.iter().enumerate() {
            println!("{}. {description}", index + 1);
        }

        print!("Option: ");
        {
            stdout().lock().flush().unwrap();
        }

        let mut option_index = String::new();
        stdin()
            .read_line(&mut option_index)
            .map_err(|x| x.to_string())?;

        println!();

        let option_index = match option_index.trim().parse::<usize>() {
            Ok(option_index) => option_index,
            Err(_) => {
                eprintln!("Invalid option index provided\n");

                continue;
            }
        };

        if option_index == 0 {
            println!("Refreshing state...\n");
            continue;
        }

        if option_index > options.len() {
            eprintln!("Invalid option index provided\n");

            continue;
        }

        let (_, func) = &options[option_index - 1];

        match func() {
            Ok(ChildProcess(result)) => {
                if let Some((child, command_hash)) = result {
                    let child_process = child.lock().unwrap();
                    let mut already_started_commands = already_started_commands.lock().unwrap();
                    let sysinfo = refresh_sysinfo(&sysinfo);
                    let alive_process = sysinfo.process(Pid::from_u32(child_process.id()));

                    if let Some(proc) = alive_process {
                        if already_started_commands.contains_key(&command_hash) {
                            println!("Process already started: {}\n", proc.cmd().join(" "));

                            continue;
                        }
                    }

                    if alive_process.is_some() {
                        already_started_commands.insert(command_hash, Arc::clone(&child));
                    }

                    if alive_process.is_none() {
                        eprintln!("Process started and died unexpectedly\n");
                    }
                }
            }
            Ok(PrintableResults(result)) => {
                let printable_results: Vec<(usize, &String)> = result.iter().enumerate().collect();

                printable_results
                    .iter()
                    .for_each(|(i, x)| println!("\t{}. {x}", i + 1));

                if !printable_results.is_empty() {
                    println!();
                }
            }
            Err(error) => eprintln!("{error}\n"),
        };
    }
}
