use std::path::{Path, PathBuf};
use std::env;
use colored::Colorize;

fn main() {
    println!("Hello, world!");
}

fn exist_config_file() -> Option<bool> {
    // checks toml format and contents.
    let path = env::current_dir();
    if path.is_err() {
        println!("{}", "failed to get current directory. try again after sometime.".red());
        return None;
    }
    let mut path: PathBuf = env::current_dir().unwrap();


    Some(false)
}

