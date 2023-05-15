use std::io::{stdin, stdout, BufRead, Write};

use kube_minion::{
    self, build_options, create_kubernetes_dashboard_load_balancer, create_minikube_tunnel,
    get_sysinfo, verify_dependencies, CommandResultType::*,
};
use sysinfo::{Pid, PidExt, SystemExt};

fn main() -> Result<(), String> {
    verify_dependencies()?;

    create_kubernetes_dashboard_load_balancer().unwrap();

    create_minikube_tunnel().unwrap();

    loop {
        let options = build_options()?;

        println!("0. Refresh options");

        for (index, (description, _)) in options.iter().enumerate() {
            println!("{}. {description}", index + 1);
        }

        print!("Option: ");
        {
            stdout().lock().flush().unwrap();
        }

        let mut option_index = String::new();
        {
            stdin()
                .lock()
                .read_line(&mut option_index)
                .map_err(|x| x.to_string())?;
        }

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
                if let Some((child, _)) = result {
                    let child_process = child.lock().unwrap();
                    let sysinfo = get_sysinfo();
                    let alive_process = sysinfo.process(Pid::from_u32(child_process.id()));

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
