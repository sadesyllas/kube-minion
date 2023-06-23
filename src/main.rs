use args::Args;
use getopts::Occur;
use regex::Regex;
use std::env;
use std::io::{stdin, stdout, BufRead, Write};

use kube_minion::{
    self, build_options, create_kubernetes_dashboard_load_balancer, create_minikube_tunnel,
    print_results, run_init_file, set_dashboard_port, verify_dependencies, OptionFunc,
};

fn main() -> Result<(), String> {
    let mut args = Args::new(
        "kube-minion",
        "https://github.com/sadesyllas/kube-minion/blob/main/README.md",
    );
    args.flag("h", "help", "Print the usage menu");
    args.option(
        "p",
        "dashboard-port",
        "The port on which to expose the Kubernetes dashboard load balancer service",
        "DASHBOARD_PORT",
        Occur::Optional,
        Some(String::from("51515")),
    );
    args.option(
        "f",
        "initialization-file-path",
        "The path to an initialization file",
        "INITIALIZATION_FILE_PATH",
        Occur::Optional,
        None,
    );
    args.parse(env::args()).map_err(|x| x.to_string())?;

    if args.value_of("help").unwrap_or_default() {
        println!("{}", args.full_usage());
        return Ok(());
    }

    let dashboard_port: u16 = args
        .value_of("dashboard-port")
        .expect("The provided value is not a valid u16");
    set_dashboard_port(dashboard_port);

    verify_dependencies()?;

    match create_kubernetes_dashboard_load_balancer() {
        results @ Ok(_) => print_results(results, true, true),
        Err(error) => return Err(error),
    };

    match create_minikube_tunnel() {
        results @ Ok(_) => print_results(results, true, true),
        Err(error) => return Err(error),
    };

    let init_file_path = run_init_file(args.value_of::<String>("initialization-file-path").ok())?;

    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, || {
            println!(
                "\nSIGINT received. Please, use option 16 or option 17 to exit the application."
            );
        })
        .map_err(|x| x.to_string())?
    };

    let mut exit = false;

    #[allow(clippy::invalid_regex)] // clippy bug?
    let option_description_header_re = Regex::new(r"^\s*#\s+(?<title>.+)").unwrap();

    loop {
        if exit {
            break;
        }

        let options = build_options(init_file_path.as_ref())?;

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

        let (_, func, exit_after) = actionable_options[option_index - 1];

        exit = *exit_after;

        print_results(func(), true, true);
    }

    Ok(())
}
