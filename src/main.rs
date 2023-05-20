use regex::Regex;
use std::io::{stdin, stdout, BufRead, Write};

use kube_minion::{
    self, build_options, create_kubernetes_dashboard_load_balancer, create_minikube_tunnel,
    print_results, run_init_file, verify_dependencies, OptionFunc,
};

fn main() -> Result<(), String> {
    println!("Documentation URL: https://github.com/sadesyllas/kube-minion/blob/main/README.md");

    verify_dependencies()?;

    create_kubernetes_dashboard_load_balancer().unwrap();

    create_minikube_tunnel().unwrap();

    run_init_file()?;

    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, || {
            println!(
                "\nSIGINT received. Please, use option 16 or option 17 to exit the application."
            );
        })
        .map_err(|x| x.to_string())?
    };

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

        let actionable_options: Vec<&(String, OptionFunc, bool)> = options
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

        if option_index > actionable_options.len() {
            eprintln!("Invalid option index provided");
            continue;
        }

        let (_, func, exit_after) = actionable_options.iter().nth(option_index - 1).unwrap();

        exit = *exit_after;

        print_results(func(), true, true);
    }

    Ok(())
}
