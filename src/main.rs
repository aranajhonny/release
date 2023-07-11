use git2::{Error, Repository};
use std::env;
use std::fs;
use std::path::Path;
use std::process::{exit, Command};

fn main() -> Result<(), Error> {
    
    setup_http()?;

    let url = "https://github.com/membrane-io/directory.git";
    let repo = match Repository::clone(url, "directory") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    for mut sub in repo.submodules()? {
        sub.update(true, None)?;
        println!("{}", sub.name().unwrap());
    }

    let membrane_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join("membrane");

    let source_folder = env::current_dir()
        .expect("Failed to get current directory")
        .join("directory");

    if let Err(err) = copy_folder(&source_folder, &membrane_dir) {
        panic!("Failed to copy folder: {}", err);
    }

    println!("Folder copied successfully!");

    let entries: Vec<_> = match fs::read_dir(&membrane_dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("Error reading directory: {}", err);
            exit(1);
        }
    }
    .collect();

    for entry in &entries[..] {
        if let Ok(entry) = entry {
            // Check if the entry is a directory
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    // Extract the subfolder name
                    let program = match entry.file_name().into_string() {
                        Ok(name) => name,
                        Err(_) => {
                            eprintln!("Error extracting folder name");
                            continue;
                        }
                    };

                    let package_json_path = entry.path().join("package.json");
                    if package_json_path.exists() {
                        // Run the `yarn` command in the subfolder
                        yarn_install(&program)
                    }
                    mctl_update(&program);
                    mctl_test(&program);
                }
            }
        }
    }

    // Separate iteration for running tests
    for entry in &entries[..] {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let program = entry.file_name();

                    let package_json_path = entry.path().join("package.json");
                    if package_json_path.exists() {
                        // Run the `mctl test` command in the subfolder
                        let mctl_test_command = Command::new("mctl")
                            .arg("test")
                            .arg(program)
                            .current_dir(&entry.path())
                            .spawn();

                        match mctl_test_command {
                            Ok(mut child) => {
                                if let Err(err) = child.wait() {
                                    eprintln!("Error executing command: {}", err);
                                }
                            }
                            Err(err) => {
                                eprintln!("Error spawning command: {}", err);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn setup_http() -> Result<(), Error> {
    let url = "https://github.com/juancampa/membrane-http-program.git";

    let membrane_dir = dirs::home_dir()
    .expect("Failed to get home directory")
    .join("membrane")
    .join("http");
  
    match Repository::clone(url, "http") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    let source_folder = env::current_dir()
        .expect("Failed to get current directory")
        .join("http");

    if let Err(err) = copy_folder(&source_folder, &membrane_dir) {
        panic!("Failed to copy folder: {}", err);
    }

    mctl_update("http");
    Ok(())
}

fn mctl_update(program: &str) {
    let command = Command::new("mctl").arg("update").arg(program).spawn();

    match command {
        Ok(mut child) => {
            if let Err(err) = child.wait() {
                eprintln!("Error executing command: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Error spawning command: {}", err);
            exit(1);
        }
    }
}

fn mctl_test(program: &str) {
    let command = Command::new("mctl").arg("test").arg(program).spawn();

    match command {
        Ok(mut child) => {
            if let Err(err) = child.wait() {
                eprintln!("Error executing command: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Error spawning command: {}", err);
            exit(1);
        }
    }
}

fn yarn_install(program: &str) {
    let subfolder_path = format!("./{}", program);
    let command_yarn = Command::new("yarn").current_dir(subfolder_path).spawn();

    println!("Running yarn in {}", program);

    match command_yarn {
        Ok(mut child) => {
            if let Err(err) = child.wait() {
                eprintln!("Error executing yarn command: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Error spawning yarn command: {}", err);
            exit(1);
        }
    }
}

fn copy_folder(source: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !destination.exists() {
        fs::create_dir_all(&destination)?;
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if entry_path.is_dir() {
            copy_folder(&entry_path, &destination_path)?;
        } else {
            fs::copy(&entry_path, &destination_path)?;
        }
    }

    Ok(())
}
