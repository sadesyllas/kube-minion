use regex::Regex;
use std::io::{stdin, stdout, BufRead, Read, Write};
use std::{process, sync, thread};

use kube_minion::{
    self, build_options, clean_up_and_exit, create_kubernetes_dashboard_load_balancer,
    create_minikube_tunnel, print_results, run_init_file, verify_dependencies,
    CommandResultType::*, OptionFunc,
};

fn main() -> Result<(), String> {
    verify_dependencies()?;

    create_kubernetes_dashboard_load_balancer().unwrap();

    create_minikube_tunnel().unwrap();

    run_init_file()?;

    let (tx, rx) = sync::mpsc::channel();

    ctrlc::set_handler(move || tx.send(true).unwrap()).unwrap();

    thread::spawn(move || {
        if let Ok(true) = rx.recv() {
            println!();

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
        for (description, _) in options.iter() {
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

        let actuable_options: Vec<&(String, OptionFunc)> = options
            .iter()
            .filter(|(description, _)| !option_description_header_re.is_match(description))
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

        let (_, func) = actuable_options.iter().nth(option_index - 1).unwrap();

        match func() {
            Ok(ChildProcess(Some((child, exit_status)))) => {
                let mut child = child.lock().unwrap();
                let mut output = String::new();

                if exit_status.success() {
                    child
                        .stdout
                        .take()
                        .unwrap()
                        .read_to_string(&mut output)
                        .unwrap();

                    let output = output.trim();

                    if !output.is_empty() {
                        println!("{output}");
                    }
                } else {
                    child
                        .stderr
                        .take()
                        .unwrap()
                        .read_to_string(&mut output)
                        .unwrap();

                    let output = output.trim();

                    if !output.is_empty() {
                        eprintln!("{output}");
                    }
                }
            }
            Ok(ChildProcess(None)) => (),
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
            Err(error) => eprintln!("{error}"),
        };
    }

    Ok(())
}
