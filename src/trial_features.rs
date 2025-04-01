extern crate walkdir;
use walkdir::WalkDir;
use std::env;
use std::io;
use std::fs::{File, create_dir_all, remove_file, remove_dir_all, metadata};
use std::process::{Command};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let home_dir = env::var("HOME").expect("Failed to get HOME directory");
    let mut pwd = home_dir.clone();
    println!("{}", pwd);
    let mut choice;
    loop {
        list_all(pwd.clone());
        choice = display_menu();
        if choice == -1 {
            continue;
        } else if choice == 0 && pwd != home_dir {
            pwd = go_back(pwd); 
        } else {
            let mut is_dir = false;
            let mut num = String::new();
            println!("Enter 1 - dir and 2 - file: ");
            io::stdin().read_line(&mut num).expect("Not valid format");
            match num.trim().parse::<i32>() {
                Ok(val) => {
                    if val == 1 {
                        is_dir = true;
                    }
                },
                Err(e) => println!("{}", e),
            }

            let mut name = String::new();
            println!("Enter Name: ");
            io::stdin().read_line(&mut name).expect("Not valid format");
            pwd = follow_operation(pwd, name.trim(), is_dir, choice);
        }
    }
}

fn display_menu() -> i32 {
    let mut choice = String::new();
    println!("Choose Operation: ");
    println!("1. Open");
    println!("2. Create");
    println!("3. Remove ");
    println!("4. Print Info");
    println!("0. Go Back");
    println!("Choice: ");
    
    io::stdin().read_line(&mut choice).expect("Nothing found");
    match choice.trim().parse::<i32>() {
        Ok(num) => return num,
        Err(e) => return -1,
    }
}

fn list_all(dir: String) {
    for entry in WalkDir::new(dir).max_depth(1) {
        match entry {
            Ok(entry) => {
                if let Some(name) = entry.file_name().to_str() {
                    if name.chars().nth(0) != Some('.') {
                        println!("{}", name);
                    }
                }
            },
            Err(e) => println!("{e}"),
        }
    }
}

fn go_back(current_dir: String) -> String {
    let path = std::path::Path::new(&current_dir);
    if let Some(parent) = path.parent() {
        return parent.to_string_lossy().into_owned();
    }
    current_dir
}

fn follow_operation(pwd: String, name: &str, is_dir: bool, choice: i32) -> String {
    println!("Operation: {}, Name: {}, Is Directory: {}", choice, name, is_dir);
    let path = Path::new(&pwd).join(name); 

    match choice {
        1 => open(path, is_dir),               // Open operation
        2 => { create(&path, is_dir); return pwd },    // Create operation
        3 => { remove(&path, is_dir); return pwd },    // Remove operation
        4 => { print_info(&path); return pwd },       // Print info operation
        _ => {
            println!("Invalid operation!");
            return pwd;
        },
    }
}

fn open(path: PathBuf, is_dir: bool) -> String {
    if path.exists() {
        if path.is_dir() {
            println!("Opening directory: {}", path.display());
            // Command::new("open")
            //     .arg(&path)
            //     .spawn()
            //     .expect("Failed to open directory");

            // If it's a directory, update the pwd to the new directory
            return path.to_str().unwrap_or("").to_string();
        } else {
            println!("Opening file: {}", path.display());
            Command::new("open")
                .arg(&path)
                .spawn()
                .expect("Failed to open file");
        }
    } else {
        println!("The path does not exist.");
    }

    
    path.to_str().unwrap_or("").to_string()
}

fn create(path: &Path, is_dir: bool) {
    if is_dir {
        if create_dir_all(path).is_ok() {
            println!("Directory '{}' created successfully.", path.display());
        } else {
            println!("Failed to create directory '{}'.", path.display());
        }
    } else {
        if File::create(path).is_ok() {
            println!("File '{}' created successfully.", path.display());
        } else {
            println!("Failed to create file '{}'.", path.display());
        }
    }
}

fn remove(path: &Path, is_dir: bool) {
    if path.exists() {
        if is_dir {
            if remove_dir_all(path).is_ok() {
                println!("Directory '{}' removed successfully.", path.display());
            } else {
                println!("Failed to remove directory '{}'.", path.display());
            }
        } else {
            if remove_file(path).is_ok() {
                println!("File '{}' removed successfully.", path.display());
            } else {
                println!("Failed to remove file '{}'.", path.display());
            }
        }
    } else {
        println!("The path does not exist.");
    }
}

fn print_info(path: &Path) {
    if path.exists() {
        match metadata(path) {
            Ok(meta) => {
                println!("Path: {}", path.display());
                if path.is_dir() {
                    println!("Type: Directory");
                } else {
                    println!("Type: File");
                }
                if let Ok(modified) = meta.modified() {
                    let duration = modified.duration_since(UNIX_EPOCH).unwrap();
                    println!("Last modified: {} seconds ago", duration.as_secs());
                }
                println!("Size: {} bytes", meta.len());
            }
            Err(e) => {
                println!("Error fetching metadata: {}", e);
            }
        }
    } else {
        println!("The path does not exist.");
    }
}