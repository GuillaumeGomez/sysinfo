use sysinfo::{RefreshKind, System, SystemExt};

fn kibi_to_mibi(value: u64) -> u64 {
    value / 1024
}

fn kibi_to_gibi(value: u64) -> f64 {
    value as f64 / 1024. / 1024.
}

fn print_value(value: u64) {
    println!("{:>8}  {:>5}  {:>6.3}", value, kibi_to_mibi(value), kibi_to_gibi(value));
}

fn main() {
    let system = System::new_with_specifics(RefreshKind::new().with_memory());

    /*
        self.mem_total
        self.mem_available
        self.swap_total
        self.swap_used
    */
    let total_memory = system.total_memory();
    let free_memory = system.free_memory();
    let total_swap = system.total_swap();
    let used_swap = system.used_swap();

    println!();
    print!("total_memory = "); print_value(total_memory);
    print!("total_swap =   "); print_value(total_swap);
    println!();
    print!("total_total =  "); print_value(total_memory + total_swap);
    println!();
    print!("free_memory =  "); print_value(free_memory);
    print!("free_swap =    "); print_value(total_swap - used_swap);
    println!();
    print!("free_total =   "); print_value(free_memory + (total_swap - used_swap));
    println!();
}
