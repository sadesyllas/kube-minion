use std::io::{stdin, stdout, BufRead, Read, Write};

use kube_minion::{
    self, build_options, create_kubernetes_dashboard_load_balancer, create_minikube_tunnel,
    verify_dependencies, CommandResultType::*,
};

fn main() -> Result<(), String> {
    verify_dependencies()?;

    create_kubernetes_dashboard_load_balancer().unwrap();

    create_minikube_tunnel().unwrap();

    let mut exit = false;

    loop {
        if exit {
            break;
        }

        let options = build_options()?;

        println!("Options:");
        println!("\t0. Refresh options");

        for (index, (description, _)) in options.iter().enumerate() {
            println!("\t{}. {description}", index + 1);
        }

        print!("\tOption: ");
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

        let option_index = match option_index.trim().parse::<usize>() {
            Ok(option_index) => option_index,
            Err(_) => {
                eprintln!("Invalid option index provided");

                continue;
            }
        };

        if option_index == 0 {
            println!("Refreshing state...");
            continue;
        }

        if option_index == options.len() {
            exit = true;
        }

        if option_index > options.len() {
            eprintln!("Invalid option index provided");
            continue;
        }

        let (_, func) = &options[option_index - 1];

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
