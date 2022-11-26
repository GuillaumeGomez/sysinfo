use std::{fs, io, os::linux::fs::MetadataExt};

fn main() -> io::Result<()> {
    let meta = fs::metadata("/home/ubuntu")?;
    println!("{meta:?}");
    println!("{}", meta.st_dev());
    Ok(())
}