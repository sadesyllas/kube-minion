use crate::load_balancer::{create_load_balancer, delete_load_balancer};
use crate::minikube_mount::{create_minikube_mount, delete_minikube_mount};
use crate::socat_tunnel::{create_socat_tunnel, delete_socat_tunnel, set_default_connect_host};
use crate::CommandResultType::PrintableResults;
use crate::{flush_output, print_results, CommandExecutionResult, OptionFunc};
use json_comments::StripComments;
use std::fs::File;
use std::io::Read;
use std::{env, fs};

pub fn run_init_file(path: Option<String>) -> Result<Option<String>, String> {
    let init_file_path = get_init_file_path(path);

    if init_file_path.is_none() {
        return Ok(None);
    }

    let init_file_path = init_file_path.unwrap();

    println!("Found initialization file: {init_file_path}");

    let init_config = parse_init_config_json(&init_file_path);

    if let Some(load_balancers) = parse_load_balancers(&init_config) {
        println!("Processing initialization file section: loadBalancers");

        for LoadBalancerConfig {
            namespace,
            resource_type,
            name,
            port,
            target_port,
        } in load_balancers
        {
            print_results(
                create_load_balancer(&namespace, &resource_type, &name, port, target_port),
                true,
                true,
            );
            flush_output();
        }
    }

    if let Some(socat_tunnels) = parse_socat_tunnels(&init_config) {
        println!("Processing initialization file section: socatTunnels");

        for SocatTunnelConfig {
            protocol,
            listening_port,
            connect_host,
            connect_port,
        } in socat_tunnels
        {
            print_results(
                create_socat_tunnel(&protocol, listening_port, &connect_host, connect_port),
                true,
                true,
            );
            flush_output();
        }
    }

    if let Some(minikube_mounts) = parse_minikube_mounts(&init_config) {
        println!("Processing initialization file section: minikubeMounts");

        for MinikubeMountConfig {
            host_path,
            minikube_path,
        } in minikube_mounts
        {
            print_results(
                create_minikube_mount(&host_path, &minikube_path),
                true,
                true,
            );
            flush_output();
        }
    }

    if let Some(default_socat_connect_host) = init_config.get("defaultSocatConnectHost") {
        let default_socat_connect_host = default_socat_connect_host
            .as_str()
            .expect("defaultSocatConnectHost must be a valid JSON string value")
            .to_string();

        println!("{}", set_default_connect_host(default_socat_connect_host));
        flush_output();
    }

    Ok(Some(init_file_path))
}

pub fn build_clean_up_init_file_option(
    init_file_path: &str,
) -> Result<(String, OptionFunc, bool), String> {
    let init_file_path = init_file_path.to_string();

    Ok((
        String::from("Clean up initialization file configuration and exit"),
        Box::new(move || clean_up_init_file(init_file_path.clone())),
        true,
    ))
}

struct LoadBalancerConfig {
    namespace: String,
    resource_type: String,
    name: String,
    port: u16,
    target_port: u16,
}

struct SocatTunnelConfig {
    protocol: String,
    listening_port: u16,
    connect_host: String,
    connect_port: u16,
}

struct MinikubeMountConfig {
    host_path: String,
    minikube_path: String,
}

fn clean_up_init_file(init_file_path: String) -> CommandExecutionResult {
    let init_config = parse_init_config_json(&init_file_path);

    if let Some(load_balancers) = parse_load_balancers(&init_config) {
        println!("Cleaning up configuration from initialization file section: loadBalancers");

        for LoadBalancerConfig {
            namespace,
            name,
            port,
            target_port,
            ..
        } in load_balancers
        {
            let name = format!("{name}-{port}-{target_port}-lb");
            print_results(delete_load_balancer(&namespace, &name), true, true);
            flush_output();
        }
    }

    if let Some(socat_tunnels) = parse_socat_tunnels(&init_config) {
        println!("Cleaning up configuration from initialization file section: socatTunnels");

        for SocatTunnelConfig {
            listening_port,
            connect_host,
            connect_port,
            ..
        } in socat_tunnels
        {
            print_results(
                delete_socat_tunnel(listening_port, &connect_host, connect_port),
                true,
                true,
            );
            flush_output();
        }
    }

    if let Some(minikube_mounts) = parse_minikube_mounts(&init_config) {
        println!("Cleaning up configuration from initialization file section: minikubeMounts");

        for MinikubeMountConfig {
            host_path,
            minikube_path,
        } in minikube_mounts
        {
            print_results(
                delete_minikube_mount(&host_path, &minikube_path),
                true,
                true,
            );
            flush_output();
        }
    }

    Ok(PrintableResults(None, Vec::new()))
}

fn get_init_file_path(path: Option<String>) -> Option<String> {
    let init_file_path = path.unwrap_or_else(|| {
        let init_file_environment_part = match env::var("KUBE_MINION_ENVIRONMENT") {
            Ok(envvar) => {
                println!(
                    "The KUBE_MINION_ENVIRONMENT environment variable has been set to {envvar}"
                );

                format!(".{envvar}")
            }
            Err(_) => String::new(),
        };

        format!("./kube-minion{init_file_environment_part}.json")
    });

    match fs::metadata(&init_file_path) {
        Ok(metadata) if metadata.is_file() => Some(init_file_path),
        _ => None,
    }
}

fn parse_init_config_json(init_file_path: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut init_file_content = String::new();

    File::open(init_file_path)
        .unwrap()
        .read_to_string(&mut init_file_content)
        .unwrap();

    let init_file_content_reader = StripComments::new(init_file_content.as_bytes());

    #[allow(clippy::expect_fun_call)] // clippy bug?
    let init_config: serde_json::Value = serde_json::from_reader(init_file_content_reader)
        .expect(&format!("{init_file_path} is not valid JSON"));

    init_config
        .as_object()
        .expect("The initial configuration is not a valid JSON object")
        .to_owned()
}

fn get_json_string(
    json: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    default: Option<&str>,
) -> String {
    match json.get(key) {
        Some(value) => {
            match value.as_str() {
                Some(value) => String::from(value),
                None => panic!("{key} must be a valid JSON string value"),
            }
        }
        None if let Some(default) = default => String::from(default),
        None => panic!("{key} requires a JSON string value"),
    }
}

fn get_json_u16(
    json: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    default: Option<u16>,
) -> u16 {
    match json.get(key) {
        Some(value) => {
            match value.as_u64() {
                Some(value) => {
                    if value == 0 || value > u16::MAX as u64 {
                        panic!("{key} must be between 1 and {}", u16::MAX);
                    }

                    value as u16
                },
                None => panic!("{key} must be a valid JSON integer value"),
            }
        }
        None if let Some(default) = default => default,
        None => panic!("{key} requires a JSON integer value"),
    }
}

fn parse_load_balancers(
    init_config: &serde_json::Map<String, serde_json::Value>,
) -> Option<Vec<LoadBalancerConfig>> {
    if let Some(load_balancer_specs) = init_config.get("loadBalancers") {
        let mut load_balancers: Vec<LoadBalancerConfig> = Vec::new();

        for load_balancer in load_balancer_specs
            .as_array()
            .expect("The loadBalancers key requires an array of load balancer specifications")
        {
            let load_balancer = load_balancer
                .as_object()
                .expect("A load balancer specification must be a valid JSON object");

            let namespace = get_json_string(load_balancer, "namespace", Some("default"));
            let resource_type = get_json_string(load_balancer, "resourceType", Some("services"));
            let name = get_json_string(load_balancer, "name", None);
            let port = get_json_u16(load_balancer, "port", None);
            let target_port = get_json_u16(load_balancer, "targetPort", Some(port));

            load_balancers.push(LoadBalancerConfig {
                namespace,
                resource_type,
                name,
                port,
                target_port,
            });
        }

        return Some(load_balancers);
    }

    None
}

fn parse_socat_tunnels(
    init_config: &serde_json::Map<String, serde_json::Value>,
) -> Option<Vec<SocatTunnelConfig>> {
    if let Some(socat_tunnel_specs) = init_config.get("socatTunnels") {
        let mut socat_tunnels: Vec<SocatTunnelConfig> = Vec::new();

        for socat_tunnel in socat_tunnel_specs
            .as_array()
            .expect("The socatTunnels key requires an array of socat tunnel specifications")
        {
            let socat_tunnel = socat_tunnel
                .as_object()
                .expect("A socat tunnel specification must be a valid JSON object");

            let protocol = get_json_string(socat_tunnel, "protocol", Some("tcp"))
                .trim()
                .to_lowercase();
            if protocol != "tcp" && protocol != "udp" {
                panic!("protocol must be either tcp or udp");
            }
            let listening_port = get_json_u16(socat_tunnel, "listeningPort", None);
            let connect_host = get_json_string(socat_tunnel, "connectHost", None);
            let connect_port = get_json_u16(socat_tunnel, "connectPort", None);

            socat_tunnels.push(SocatTunnelConfig {
                protocol,
                listening_port,
                connect_host,
                connect_port,
            });
        }

        return Some(socat_tunnels);
    }

    None
}

fn parse_minikube_mounts(
    init_config: &serde_json::Map<String, serde_json::Value>,
) -> Option<Vec<MinikubeMountConfig>> {
    if let Some(minikube_mount_specs) = init_config.get("minikubeMounts") {
        let mut minikube_mounts: Vec<MinikubeMountConfig> = Vec::new();

        for minikube_mount in minikube_mount_specs
            .as_array()
            .expect("The minikubeMounts key requires an array of minikube mount specifications")
        {
            let minikube_mount = minikube_mount
                .as_object()
                .expect("A minikube mount specification must be a valid JSON object");

            let host_path = get_json_string(minikube_mount, "hostPath", None);
            let minikube_path = get_json_string(minikube_mount, "minikubePath", None);

            minikube_mounts.push(MinikubeMountConfig {
                host_path,
                minikube_path,
            });
        }

        return Some(minikube_mounts);
    }

    None
}
