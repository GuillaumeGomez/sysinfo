use reedline::{
    DefaultCompleter, DefaultPrompt, DefaultPromptSegment, DescriptionMode, EditCommand, Emacs,
    ExampleHighlighter, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use std::io;
use std::str::FromStr;
use sysinfo::{Components, Disks, Groups, Networks, Pid, SUPPORTED_SIGNALS, System, Users};

struct Context<'a> {
    args: Vec<&'a str>,
    sys: &'a mut System,
    networks: &'a mut Networks,
    disks: &'a mut Disks,
    components: &'a mut Components,
    users: &'a mut Users,
}

type CommandHandler = fn(&mut Context) -> bool;

struct CommandDef {
    name: &'static str,
    handler: CommandHandler,
}

const COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "help",
        handler: |_context| {
            print_help();
            false
        },
    },
    CommandDef {
        name: "quit",
        handler: |_context| true,
    },
    CommandDef {
        name: "exit",
        handler: |_context| true,
    },
    CommandDef {
        name: "refresh_disks",
        handler: |context| {
            println!("Refreshing disk list...");
            context.disks.refresh(true);
            println!("Done.");
            false
        },
    },
    CommandDef {
        name: "refresh_users",
        handler: |context| {
            println!("Refreshing user list...");
            context.users.refresh();
            println!("Done.");
            false
        },
    },
    CommandDef {
        name: "refresh_networks",
        handler: |context| {
            println!("Refreshing network list...");
            context.networks.refresh(true);
            println!("Done.");
            false
        },
    },
    CommandDef {
        name: "refresh_components",
        handler: |context| {
            println!("Refreshing component list...");
            context.components.refresh(true);
            println!("Done.");
            false
        },
    },
    CommandDef {
        name: "refresh_cpu",
        handler: |context| {
            println!("Refreshing CPUs...");
            context.sys.refresh_cpu_all();
            println!("Done.");
            false
        },
    },
    CommandDef {
        name: "all",
        handler: |context| {
            for (pid, proc_) in context.sys.processes() {
                println!(
                    "{}:{} status={:?}",
                    pid,
                    proc_.name().to_string_lossy(),
                    proc_.status()
                );
            }
            false
        },
    },
    CommandDef {
        name: "signals",
        handler: |_context| {
            for (nb, sig) in SUPPORTED_SIGNALS.iter().enumerate() {
                println!("{:2}:{sig:?}", nb + 1);
            }
            false
        },
    },
    CommandDef {
        name: "cpus",
        handler: |context| {
            println!(
                "number of physical cores: {}",
                System::physical_core_count()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "Unknown".to_owned()),
            );
            println!("total CPU usage: {}%", context.sys.global_cpu_usage());
            for cpu in context.sys.cpus() {
                println!("{cpu:?}");
            }
            false
        },
    },
    CommandDef {
        name: "memory",
        handler: |context| {
            println!(
                "total memory:     {: >10} KB",
                context.sys.total_memory() / 1_000
            );
            println!(
                "available memory: {: >10} KB",
                context.sys.available_memory() / 1_000
            );
            println!(
                "used memory:      {: >10} KB",
                context.sys.used_memory() / 1_000
            );
            println!(
                "total swap:       {: >10} KB",
                context.sys.total_swap() / 1_000
            );
            println!(
                "used swap:        {: >10} KB",
                context.sys.used_swap() / 1_000
            );
            false
        },
    },
    CommandDef {
        name: "frequency",
        handler: |context| {
            for cpu in context.sys.cpus() {
                println!("[{}] {} MHz", cpu.name(), cpu.frequency());
            }
            false
        },
    },
    CommandDef {
        name: "vendor_id",
        handler: |context| {
            println!("vendor ID: {}", context.sys.cpus()[0].vendor_id());
            false
        },
    },
    CommandDef {
        name: "brand",
        handler: |context| {
            println!("brand: {}", context.sys.cpus()[0].brand());
            false
        },
    },
    CommandDef {
        name: "load_avg",
        handler: |_context| {
            let load_avg = System::load_average();
            println!("one minute     : {}%", load_avg.one);
            println!("five minutes   : {}%", load_avg.five);
            println!("fifteen minutes: {}%", load_avg.fifteen);
            false
        },
    },
    CommandDef {
        name: "show",
        handler: |context| {
            if context.args.len() != 1 {
                println!("show command takes a pid or a name in parameter!");
                println!("example: show 1254");
                return false;
            }
            let arg = context.args[0];
            if let Ok(pid) = Pid::from_str(arg) {
                match context.sys.process(pid) {
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
                let proc_name = arg;
                for proc_ in context.sys.processes_by_name(proc_name.as_ref()) {
                    println!("==== {} ====", proc_.name().to_string_lossy());
                    println!("{proc_:?}");
                }
            }
            false
        },
    },
    CommandDef {
        name: "temperature",
        handler: |context| {
            for component in context.components.iter() {
                println!("{component:?}");
            }
            false
        },
    },
    CommandDef {
        name: "network",
        handler: |context| {
            for (interface_name, data) in context.networks.iter() {
                println!(
                    "\
{interface_name}:
  ether {}
  input data  (new / total): {} / {} B
  output data (new / total): {} / {} B",
                    data.mac_address(),
                    data.received(),
                    data.total_received(),
                    data.transmitted(),
                    data.total_transmitted(),
                );
            }
            false
        },
    },
    CommandDef {
        name: "kill",
        handler: |context| {
            if context.args.len() != 2 {
                println!("kill command takes the pid and a signal number in parameter!");
                println!("example: kill 1254 9");
                return false;
            }
            let pid_str = context.args[0];
            let signal_str = context.args[1];
            let Ok(pid) = Pid::from_str(pid_str) else {
                eprintln!("Expected a number for the PID, found {:?}", pid_str);
                return false;
            };
            let Ok(signal) = usize::from_str(signal_str) else {
                eprintln!("Expected a number for the signal, found {:?}", signal_str);
                return false;
            };
            let Some(signal) = SUPPORTED_SIGNALS.get(signal) else {
                eprintln!(
                    "No signal matching {signal}. Use the `signals` command to get the \
                     list of signals.",
                );
                return false;
            };
            match context.sys.process(pid) {
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
            }
            false
        },
    },
    CommandDef {
        name: "disks",
        handler: |context| {
            for disk in &*context.disks {
                println!("{disk:?}");
            }
            false
        },
    },
    CommandDef {
        name: "users",
        handler: |context| {
            for user in &*context.users {
                println!("{:?} => {:?}", user.name(), user.groups());
            }
            false
        },
    },
    CommandDef {
        name: "groups",
        handler: |_context| {
            for group in Groups::new_with_refreshed_list().list() {
                println!("{group:?}");
            }
            false
        },
    },
    CommandDef {
        name: "boot_time",
        handler: |_context| {
            println!("{} seconds", System::boot_time());
            false
        },
    },
    CommandDef {
        name: "uptime",
        handler: |_context| {
            let up = System::uptime();
            let mut uptime = up;
            let days = uptime / 86400;
            uptime -= days * 86400;
            let hours = uptime / 3600;
            uptime -= hours * 3600;
            let minutes = uptime / 60;
            println!("{days} days {hours} hours {minutes} minutes ({up} seconds in total)");
            false
        },
    },
    CommandDef {
        name: "pid",
        handler: |_context| {
            println!(
                "PID: {}",
                sysinfo::get_current_pid().expect("failed to get PID")
            );
            false
        },
    },
    CommandDef {
        name: "system",
        handler: |_context| {
            println!(
                "System name:              {}\n\
                 System kernel version:    {}\n\
                 System OS version:        {}\n\
                 System OS (long) version: {}\n\
                 System host name:         {}\n\
                 System kernel:            {}",
                System::name().unwrap_or_else(|| "<unknown>".to_owned()),
                System::kernel_version().unwrap_or_else(|| "<unknown>".to_owned()),
                System::os_version().unwrap_or_else(|| "<unknown>".to_owned()),
                System::long_os_version().unwrap_or_else(|| "<unknown>".to_owned()),
                System::host_name().unwrap_or_else(|| "<unknown>".to_owned()),
                System::kernel_long_version(),
            );
            false
        },
    },
    CommandDef {
        name: "refresh",
        handler: |context| {
            if context.args.is_empty() {
                println!("Getting processes' information...");
                context.sys.refresh_all();
                println!("Done.");
            } else if context.args.len() == 1 {
                println!("Getting process' information...");
                if let Ok(pid) = Pid::from_str(context.args[0]) {
                    if context
                        .sys
                        .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true)
                        != 0
                    {
                        println!("Process `{pid}` updated successfully");
                    } else {
                        println!("Process `{pid}` couldn't be updated...");
                    }
                } else {
                    println!("Invalid [pid] received...");
                }
            } else {
                println!("refresh command takes no arguments or a single pid");
            }
            false
        },
    },
];

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
}

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

= Users and groups commands =

groups             : displays all groups
users              : displays all users and their groups

= System commands =

boot_time          : displays system boot time
load_avg           : displays system load average
system             : displays system information (such as name, version and hostname)
uptime             : displays system uptime

= Extra components commands =

disks              : displays disks' information
memory             : displays memory state
network            : displays network' information
temperature        : displays components' temperature"
    );
}

fn interpret_input<'a>(input: &'a str, context: &mut Context<'a>) -> bool {
    let args: Vec<&str> = input.split_whitespace().collect();
    if args.is_empty() {
        return false;
    }
    let cmd_name = args[0];
    context.args = args[1..].to_vec();
    for cmd in COMMANDS {
        if cmd.name == cmd_name {
            return (cmd.handler)(context);
        }
    }
    println!(
        "\"{cmd_name}\": Unknown command. Enter 'help' if you want to get the commands' \
         list.",
    );
    false
}

fn main() -> io::Result<()> {
    let commands: Vec<String> = COMMANDS.iter().map(|c| c.name.to_string()).collect();

    let completer = Box::new({
        let mut completions = DefaultCompleter::with_inclusions(&['-', '_']);
        completions.insert(commands.clone());
        completions
    });

    // Use the interactive menu to select options from the completer
    let ide_menu = IdeMenu::default()
        .with_name("completion_menu")
        .with_min_completion_width(0)
        .with_max_completion_width(50)
        .with_max_completion_height(u16::MAX)
        .with_padding(0)
        .with_cursor_offset(0)
        .with_description_mode(DescriptionMode::PreferRight)
        .with_min_description_width(0)
        .with_max_description_width(50)
        .with_description_offset(1)
        .with_correct_cursor_pos(false);

    let completion_menu = Box::new(ide_menu);

    let mut keybindings = default_emacs_keybindings();
    add_menu_keybindings(&mut keybindings);

    let edit_mode = Box::new(Emacs::new(keybindings));

    let mut line_editor = Reedline::create()
        .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Empty,
        DefaultPromptSegment::CurrentDateTime,
    );

    // Initialize sysinfo components
    println!("Getting system information...");
    let mut system = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();
    let mut disks = Disks::new_with_refreshed_list();
    let mut components = Components::new_with_refreshed_list();
    let mut users = Users::new_with_refreshed_list();
    println!("Done.");

    println!("To get the commands' list, enter 'help'.");

    loop {
        let sig = line_editor.read_line(&prompt)?;
        match sig {
            Signal::Success(buffer) => {
                let should_quit = interpret_input(
                    buffer.as_ref(),
                    &mut Context {
                        args: vec![],
                        sys: &mut system,
                        networks: &mut networks,
                        disks: &mut disks,
                        components: &mut components,
                        users: &mut users,
                    },
                );
                if should_quit {
                    return Ok(());
                }
            }
            Signal::CtrlD | Signal::CtrlC => {
                println!("\nAborted!");
                return Ok(());
            }
        }
    }
}
