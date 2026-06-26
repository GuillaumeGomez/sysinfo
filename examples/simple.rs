// Take a look at the license at the top of the repository in the LICENSE file.

#![crate_type = "bin"]
#![allow(unused_must_use, non_upper_case_globals)]
#![allow(clippy::manual_range_contains)]

use std::io::{self, BufRead, Write};
use std::str::FromStr;
use sysinfo::{
    Components, Disks, Gpus, Groups, Motherboard, Networks, Pid, Product, SUPPORTED_SIGNALS,
    System, Users,
};

fn print_help() {
    println!(
        "\
== Help menu ==

help               : shows this menu
quit               : exits the program

= Refresh commands =

refresh            : reloads all processes information
refresh [pid]      : reloads corresponding process information
refresh_components : reloads components information
refresh_cpu        : reloads CPU information
refresh_disks      : reloads disks information
refresh_networks   : reloads networks information
refresh_users      : reloads users information

= Process commands =

all                : displays all process name and pid
kill [pid] [signal]: sends [signal] to the process with this [pid]. To get the [signal] number, use the `signals` command.
show [pid | name]  : shows information of the given process corresponding to [pid | name]
signals            : shows the available signals
pid                : displays this example's PID

= CPU commands =

brand              : displays CPU brand
cpus               : displays CPUs state
frequency          : displays CPU frequency
vendor_id          : displays CPU vendor id

= GPU commands =

gpus               : displays all GPUs

= Users and groups commands =

groups             : displays all groups
users              : displays all users and their groups

= System commands =

boot_time          : displays system boot time
load_avg           : displays system load average
system             : displays system information (such as name, version and hostname)
uptime             : displays system uptime
motherboard        : displays motherboard information
product            : displays product information

= Extra components commands =

disks              : displays disks' information
memory             : displays memory state
network            : displays network' information
temperature        : displays components' temperature"
    );
}

fn interpret_input(
    input: &str,
    sys: Result<&mut System, &mut sysinfo::Error>,
    gpus: Result<&mut Gpus, &mut sysinfo::Error>,
    networks: &mut Networks,
    disks: &mut Disks,
    components: &mut Components,
    users: Result<&mut Users, &mut sysinfo::Error>,
) -> bool {
    match input.trim() {
        "help" => print_help(),
        "refresh_disks" => {
            println!("Refreshing disk list...");
            disks.refresh(true);
            println!("Done.");
        }
        "refresh_users" => match users {
            Ok(users) => {
                println!("Refreshing user list...");
                users.refresh();
                println!("Done.");
            }
            Err(error) => {
                println!("Users information cannot be retrieved: {error}");
            }
        },
        "refresh_networks" => {
            println!("Refreshing network list...");
            networks.refresh(true);
            println!("Done.");
        }
        "refresh_components" => {
            println!("Refreshing component list...");
            components.refresh(true);
            println!("Done.");
        }
        "refresh_cpu" => match sys {
            Ok(sys) => {
                println!("Refreshing CPUs...");
                sys.refresh_cpu_all();
                println!("Done.");
            }
            Err(error) => {
                println!("System information cannot be retrieved: {error}");
            }
        },
        "signals" => {
            for (nb, sig) in SUPPORTED_SIGNALS.iter().enumerate() {
                println!("{:2}:{sig:?}", nb + 1);
            }
        }
        "cpus" => {
            match sys {
                Ok(sys) => {
                    // Note: you should refresh a few times before using this, so that usage statistics
                    // can be ascertained
                    println!(
                        "number of physical cores: {}",
                        System::physical_core_count()
                            .map(|c| c.to_string())
                            .unwrap_or_else(|_| "Unknown".to_owned()),
                    );
                    println!("total CPU usage: {}%", sys.global_cpu_usage());
                    for cpu in sys.cpus() {
                        println!("{cpu:?}");
                    }
                }
                Err(error) => {
                    println!("System information cannot be retrieved: {error}");
                }
            }
        }
        "gpus" => match gpus {
            Ok(gpus) => {
                gpus.refresh(true);
                for gpu in gpus.list() {
                    println!(
                        "GPU (PCI: {}): Vendor: {}",
                        gpu.pci(),
                        gpu.vendor().unwrap_or("Unknown")
                    );
                    if let Some(model) = gpu.model() {
                        println!("  model: {model}");
                    }
                    match gpu.usage() {
                        Some(usage) => println!("  usage: {usage}%"),
                        None => println!("  usage: N/A"),
                    }
                    if let (Some(used), Some(total)) = (gpu.used_memory(), gpu.total_memory()) {
                        println!("  memory: {}/{} KB", used / 1_000, total / 1_000);
                    } else {
                        println!("  memory: N/A");
                    }
                }
            }
            Err(error) => {
                println!("GPU information cannot be retrieved on this system: {error}");
            }
        },
        "memory" => match sys {
            Ok(sys) => {
                println!("total memory:     {: >10} KB", sys.total_memory() / 1_000);
                println!(
                    "available memory: {: >10} KB",
                    sys.available_memory() / 1_000
                );
                println!("used memory:      {: >10} KB", sys.used_memory() / 1_000);
                println!("total swap:       {: >10} KB", sys.total_swap() / 1_000);
                println!("used swap:        {: >10} KB", sys.used_swap() / 1_000);
            }
            Err(error) => {
                println!("System information cannot be retrieved: {error}");
            }
        },
        "quit" | "exit" => return true,
        "all" => match sys {
            Ok(sys) => {
                for (pid, proc_) in sys.processes() {
                    println!(
                        "{}:{} status={:?}",
                        pid,
                        proc_.name().to_string_lossy(),
                        proc_.status()
                    );
                }
            }
            Err(error) => {
                println!("System information cannot be retrieved: {error}");
            }
        },
        "frequency" => match sys {
            Ok(sys) => {
                for cpu in sys.cpus() {
                    println!("[{}] {} MHz", cpu.name(), cpu.frequency());
                }
            }
            Err(error) => {
                println!("System information cannot be retrieved: {error}");
            }
        },
        "vendor_id" => match sys {
            Ok(sys) => {
                println!("vendor ID: {}", sys.cpus()[0].vendor_id());
            }
            Err(error) => {
                println!("System information cannot be retrieved: {error}");
            }
        },
        "brand" => match sys {
            Ok(sys) => {
                println!("brand: {}", sys.cpus()[0].brand());
            }
            Err(error) => {
                println!("System information cannot be retrieved: {error}");
            }
        },
        "load_avg" => match System::load_average() {
            Ok(load_avg) => {
                println!("one minute     : {}%", load_avg.one);
                println!("five minutes   : {}%", load_avg.five);
                println!("fifteen minutes: {}%", load_avg.fifteen);
            }
            Err(error) => {
                eprintln!("Failed to get `load_average`: {error}");
            }
        },
        e if e.starts_with("show ") => {
            let tmp: Vec<&str> = e.split(' ').filter(|s| !s.is_empty()).collect();

            if tmp.len() != 2 {
                println!("show command takes a pid or a name in parameter!");
                println!("example: show 1254");
            } else {
                match sys {
                    Ok(sys) => {
                        if let Ok(pid) = Pid::from_str(tmp[1]) {
                            match sys.process(pid) {
                                Some(p) => {
                                    println!("{:?}", *p);
                                    println!(
                                        "Files open/limit: {:?}/{:?}",
                                        p.open_files(),
                                        p.open_files_limit(),
                                    );
                                }
                                None => {
                                    println!("pid \"{pid:?}\" not found");
                                }
                            }
                        } else {
                            let proc_name = tmp[1];
                            for proc_ in sys.processes_by_name(proc_name.as_ref()) {
                                println!("==== {} ====", proc_.name().to_string_lossy());
                                println!("{proc_:?}");
                            }
                        }
                    }
                    Err(error) => {
                        println!("System information cannot be retrieved: {error}");
                    }
                }
            }
        }
        "temperature" => {
            for component in components.iter() {
                println!("{component:?}");
            }
        }
        "network" => {
            for (interface_name, data) in networks.iter() {
                println!(
                    "\
{interface_name}:
  operational state {}
  ether {}
  input data  (new / total): {} / {} B
  output data (new / total): {} / {} B",
                    data.operational_state(),
                    data.mac_address(),
                    data.received(),
                    data.total_received(),
                    data.transmitted(),
                    data.total_transmitted(),
                );
            }
        }
        "show" => {
            println!("'show' command expects a pid number or a process name");
        }
        e if e.starts_with("kill ") => {
            let tmp: Vec<&str> = e
                .split(' ')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if tmp.len() != 3 {
                println!("kill command takes the pid and a signal number in parameter!");
                println!("example: kill 1254 9");
            } else {
                let Ok(pid) = Pid::from_str(tmp[1]) else {
                    eprintln!("Expected a number for the PID, found {:?}", tmp[1]);
                    return false;
                };
                let Ok(signal) = usize::from_str(tmp[2]) else {
                    eprintln!("Expected a number for the signal, found {:?}", tmp[2]);
                    return false;
                };
                let Some(signal) = SUPPORTED_SIGNALS.get(signal) else {
                    eprintln!(
                        "No signal matching {signal}. Use the `signals` command to get the \
                         list of signals.",
                    );
                    return false;
                };

                match sys {
                    Ok(sys) => match sys.process(pid) {
                        Some(p) => {
                            if let Some(res) = p.kill_with(*signal) {
                                println!("kill: {res}");
                            } else {
                                eprintln!("kill: signal not supported on this platform");
                            }
                        }
                        None => {
                            eprintln!("pid not found");
                        }
                    },
                    Err(error) => {
                        println!("System information cannot be retrieved: {error}");
                    }
                }
            }
        }
        "disks" => {
            for disk in disks {
                println!("{disk:?}");
            }
        }
        "users" => match users {
            Ok(users) => {
                for user in users {
                    println!("{:?} => {:?}", user.name(), user.groups());
                }
            }
            Err(error) => {
                println!("Users information cannot be retrieved: {error}");
            }
        },
        "groups" => match Groups::new_with_refreshed_list() {
            Ok(groups) => {
                for group in groups.list() {
                    println!("{group:?}");
                }
            }
            Err(error) => {
                println!("Groups information cannot be retrieved: {error}");
            }
        },
        "boot_time" => match System::boot_time() {
            Ok(boot_time) => println!("{boot_time} seconds"),
            Err(error) => eprintln!("Failed to get `boot_time`: {error}"),
        },
        "uptime" => match System::uptime() {
            Ok(up) => {
                let mut uptime = up;
                let days = uptime / 86400;
                uptime -= days * 86400;
                let hours = uptime / 3600;
                uptime -= hours * 3600;
                let minutes = uptime / 60;
                println!("{days} days {hours} hours {minutes} minutes ({up} seconds in total)");
            }
            Err(error) => eprintln!("Failed to get `uptime`: {error}"),
        },
        x if x.starts_with("refresh") => {
            if x == "refresh" {
                match sys {
                    Ok(sys) => {
                        println!("Getting processes' information...");
                        sys.refresh_all();
                        println!("Done.");
                    }
                    Err(error) => {
                        println!("System information cannot be retrieved: {error}");
                    }
                }
            } else if x.starts_with("refresh ") {
                println!("Getting process' information...");
                if let Some(pid) = x
                    .split(' ')
                    .filter_map(|pid| pid.parse().ok())
                    .take(1)
                    .next()
                {
                    match sys {
                        Ok(sys) => {
                            if sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true)
                                != 0
                            {
                                println!("Process `{pid}` updated successfully");
                            } else {
                                println!("Process `{pid}` couldn't be updated...");
                            }
                        }
                        Err(error) => {
                            println!("System information cannot be retrieved: {error}");
                        }
                    }
                } else {
                    println!("Invalid [pid] received...");
                }
            } else {
                println!(
                    "\"{x}\": Unknown command. Enter 'help' if you want to get the commands' \
                     list.",
                );
            }
        }
        "pid" => {
            println!(
                "PID: {}",
                sysinfo::get_current_pid().expect("failed to get PID")
            );
        }
        "system" => {
            println!(
                "System name:              {}\n\
                 System kernel version:    {}\n\
                 System OS version:        {}\n\
                 System OS (long) version: {}\n\
                 System host name:         {}\n\
                 System kernel:            {}",
                System::name().unwrap_or_else(|_| "<unknown>".to_owned()),
                System::kernel_version().unwrap_or_else(|_| "<unknown>".to_owned()),
                System::os_version().unwrap_or_else(|_| "<unknown>".to_owned()),
                System::long_os_version().unwrap_or_else(|_| "<unknown>".to_owned()),
                System::host_name().unwrap_or_else(|_| "<unknown>".to_owned()),
                System::kernel_long_version().unwrap_or_else(|_| "<unknown>".to_owned()),
            );
        }
        "motherboard" => match Motherboard::new() {
            Ok(m) => println!("{m:#?}"),
            Err(error) => println!("Cannot retrieve motherboard information: {error:?}"),
        },
        "product" => {
            println!("{:#?}", Product);
        }
        e => {
            println!(
                "\"{e}\": Unknown command. Enter 'help' if you want to get the commands' \
                 list.",
            );
        }
    }
    false
}

fn main() {
    println!("Getting system information...");
    let mut system = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();
    let mut disks = Disks::new_with_refreshed_list();
    let mut components = Components::new_with_refreshed_list();
    let mut users = Users::new_with_refreshed_list();
    let mut gpus = Gpus::new();

    println!("Done.");
    let t_stin = io::stdin();
    let mut stin = t_stin.lock();
    let mut done = false;

    println!("To get the commands' list, enter 'help'.");
    while !done {
        let mut input = String::new();
        write!(&mut io::stdout(), "> ");
        io::stdout().flush();

        stin.read_line(&mut input);
        if input.is_empty() {
            // The string is empty, meaning there is no '\n', meaning
            // that the user used CTRL+D so we can just quit!
            println!("\nLeaving, bye!");
            break;
        }
        if (&input as &str).ends_with('\n') {
            input.pop();
        }
        done = interpret_input(
            input.as_ref(),
            system.as_mut(),
            gpus.as_mut(),
            &mut networks,
            &mut disks,
            &mut components,
            users.as_mut(),
        );
    }
}
