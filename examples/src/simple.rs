#![crate_type = "bin"]

#![feature(path)]

extern crate sysinfo;

use sysinfo::*;
use std::io::{self, BufRead};
use std::str::FromStr;
use std::io::Write;

fn print_help() -> bool {
    io::stdout().write("== Help menu ==\n".as_bytes());
    io::stdout().write("help: show this menu\n".as_bytes());
    io::stdout().write("refresh: reloads processus' information\n".as_bytes());
    io::stdout().write("show [pid]: show information of the given [pid]\n".as_bytes());
    io::stdout().write("quit: exit the program\n".as_bytes());
    false
}

fn interpret_input(input: &str, sys: &mut System) -> bool {
    match input {
        "help" => print_help(),
        "refresh" => {
            io::stdout().write("Getting processus' information...\n".as_bytes());
            sys.refresh();
            io::stdout().write("Done.\n".as_bytes());
            false
        },
        "quit" => true,
        e if e.starts_with("show ") => {
            let tmp : Vec<&str> = e.split(" ").collect();

            if tmp.len() != 2 {
                io::stdout().write("show command takes the pid in parameter !\n".as_bytes());
                io::stdout().write("example: show 1254\n".as_bytes());
            } else {
                let pid = i32::from_str(tmp.get(1).unwrap()).unwrap();

                match sys.get_processus(pid) {
                    Some(p) => io::stdout().write(format!("{:?}\n", *p).as_bytes()),
                    None => io::stdout().write("pid not found\n".as_bytes())
                };
            }
            false
        },
        e => {
            io::stdout().write(format!("\"{}\": Unknown command. Enter 'help' if you want to get the commands' list.\n", e).as_bytes());
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
        io::stdout().write("> ".as_bytes());
        io::stdout().flush();

        stin.read_line(&mut input);
        if input.as_slice().ends_with("\n") {
            input.pop();
        }
        done = interpret_input(input.as_slice(), &mut t);
    }
}