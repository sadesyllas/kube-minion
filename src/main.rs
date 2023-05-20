use regex::Regex;
use std::io::{stdin, stdout, BufRead, Write};
use std::{process, sync, thread};

use kube_minion::{
    self, build_options, clean_up, create_kubernetes_dashboard_load_balancer,
    create_minikube_tunnel, print_results, run_init_file, verify_dependencies, OptionFunc,
};

fn main() -> Result<(), String> {
    println!("Documentation URL: https://github.com/sadesyllas/kube-minion/blob/main/README.md");

    verify_dependencies()?;

    create_kubernetes_dashboard_load_balancer().unwrap();

    create_minikube_tunnel().unwrap();

    run_init_file()?;

    let (tx, rx) = sync::mpsc::channel();

    ctrlc::set_handler(move || tx.send(true).unwrap()).unwrap();

    thread::spawn(move || {
        if let Ok(true) = rx.recv() {
            println!("\nReceived Ctrl-C/SIGINT. Exiting...");

            print_results(clean_up_and_exit(), true, true);

            process::exit(0);
        }
    });

    let mut exit = false;

    let option_description_header_re = Regex::new(r"^\s*#\s+(?<title>.+)").unwrap();

    loop {
        if exit {
            break;
        }

        let options = build_options()?;

        println!("Options:");
        println!("\t0. Refresh options");

        let mut index = 0;
        for (description, _, _) in options.iter() {
            if option_description_header_re.is_match(description) {
                let title = option_description_header_re
                    .captures(description)
                    .unwrap()
                    .name("title")
                    .unwrap()
                    .as_str();

                println!("\t=========================");
                println!("\t{title}");
                println!("\t-------------------------");
            } else {
                index += 1;

                println!("\t{index}. {description}");
            }
        }

        print!("\tOption: ");
        {
            stdout().lock().flush().unwrap();
        }

        let actuable_options: Vec<&(String, OptionFunc, bool)> = options
            .iter()
            .filter(|(description, _, _)| !option_description_header_re.is_match(description))
            .collect();

        let mut option_index = String::new();
        {
            stdin()
                .lock()
                .read_line(&mut option_index)
                .map_err(|x| x.to_string())?;
        }

        let option_index = match option_index.trim().parse::<usize>() {
            Ok(option_index) => option_index,
            Err(_) => {
                eprintln!("Invalid option index provided");

                continue;
            }
        };

        if option_index == 0 {
            println!("Refreshing options...");
            continue;
        }

        if option_index == actuable_options.len() {
            exit = true;
        }

        if option_index > actuable_options.len() {
            eprintln!("Invalid option index provided");
            continue;
        }

        let (_, func, exit_after) = actuable_options.iter().nth(option_index - 1).unwrap();

        exit = *exit_after;

        print_results(func(), true, true);
    }

    Ok(())
}
