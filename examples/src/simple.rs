#![crate_type = "bin"]

 #![allow(unused_must_use, non_upper_case_globals)]

extern crate sysinfo;

use sysinfo::*;
use sysinfo::Signal::*;
use std::io::{self, BufRead};
use std::str::FromStr;
use std::io::Write;

const signals : [Signal; 31] = [Hangup, Interrupt, Quit, Illegal, Trap, Abort, Bus, FloatingPointException, Kill, User1,
    Segv, User2, Pipe, Alarm, Term, Stklft, Child, Continue, Stop, TSTP, TTIN, TTOU, Urgent, XCPU, XFSZ, VirtualAlarm,
    Profiling, Winch, IO, Power, Sys];

fn print_help() -> bool {
    write!(&mut io::stdout(), "== Help menu ==\n");
    write!(&mut io::stdout(), "help               : show this menu\n");
    write!(&mut io::stdout(), "signals            : show the available signals\n");
    write!(&mut io::stdout(), "refresh            : reloads processus' information\n");
    write!(&mut io::stdout(), "show [pid]         : show information of the given [pid]\n");
    write!(&mut io::stdout(), "kill [pid] [signal]: send [signal] to the processus with this [pid]. 0 < [signal] < 32\n");
    write!(&mut io::stdout(), "proc               : Displays proc state\n");
    write!(&mut io::stdout(), "memory             : Displays memory state\n");
    write!(&mut io::stdout(), "quit               : exit the program\n");
    false
}

fn interpret_input(input: &str, sys: &mut System) -> bool {
    match input {
        "help" => print_help(),
        "refresh" => {
            write!(&mut io::stdout(), "Getting processus' information...\n");
            sys.refresh();
            write!(&mut io::stdout(), "Done.\n");
            false
        },
        "signals" => {
            let mut nb = 1i32;

            for sig in signals.iter() {
                write!(&mut io::stdout(), "{:2}:{:?}\n", nb, sig);
                nb += 1;
            }
            false
        },
        "proc" => {
            let procs = sys.get_process_list();

            write!(&mut io::stdout(), "total process usage: {}%\n", procs[0].get_cpu_usage());
            for i in 1..procs.len() {
                write!(&mut io::stdout(), "{:?}\n", procs[i]);
            }
            false
        },
        "memory" => {
            write!(&mut io::stdout(), "total memory: {} kB\n", sys.get_total_memory());
            write!(&mut io::stdout(), "used memory : {} kB\n", sys.get_used_memory());
            write!(&mut io::stdout(), "total swap  : {} kB\n", sys.get_total_swap());
            write!(&mut io::stdout(), "used swap : {} kB\n", sys.get_used_swap());
            false
        },
        "quit" => true,
        e if e.starts_with("show ") => {
            let tmp : Vec<&str> = e.split(" ").collect();

            if tmp.len() != 2 {
                write!(&mut io::stdout(), "show command takes the pid in parameter !\n");
                write!(&mut io::stdout(), "example: show 1254\n");
            } else {
                let pid = i64::from_str(tmp.get(1).unwrap()).unwrap();

                match sys.get_processus(pid) {
                    Some(p) => write!(&mut io::stdout(), "{:?}\n", *p),
                    None => write!(&mut io::stdout(), "pid not found\n")
                };
            }
            false
        },
        e if e.starts_with("kill ") => {
            let tmp : Vec<&str> = e.split(" ").collect();

            if tmp.len() != 3 {
                write!(&mut io::stdout(), "kill command takes the pid and a signal number in parameter !\n");
                write!(&mut io::stdout(), "example: kill 1254 9\n");
            } else {
                let pid = i64::from_str(tmp.get(1).unwrap()).unwrap();
                let signal = i32::from_str(tmp.get(2).unwrap()).unwrap();

                if signal < 1 || signal > 31 {
                    write!(&mut io::stdout(), "Signal must be between 0 and 32 ! See the signals list with the signals command\n");
                } else {
                    match sys.get_processus(pid) {
                        Some(p) => {
                            write!(&mut io::stdout(), "kill: {}\n", p.kill(*signals.get(signal as usize - 1).unwrap()));
                        },
                        None => {
                            write!(&mut io::stdout(), "pid not found\n");
                        }
                    };
                }
            }
            false
        },
        e => {
            write!(&mut io::stdout(), "\"{}\": Unknown command. Enter 'help' if you want to get the commands' list.\n", e);
            false
        }
    }
}

fn main() {
    println!("Getting processus' information...");
    let mut t = System::new();
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
        if (&input as &str).ends_with("\n") {
            input.pop();
        }
        done = interpret_input(input.as_ref(), &mut t);
    }
}