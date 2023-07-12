use git2::{Error, Repository};
use reqwest::{Client, Error as HttpError, StatusCode};
use serde::Serialize;
use serde_json::json;
use std::io::Write;
use std::{
    env, fs,
    fs::File,
    io,
    path::Path,
    process::{exit, Command},
};

#[derive(Serialize)]
struct TestResult {
    program: String,
    success: bool,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
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

    let _ = copy_folder(&source_folder, &membrane_dir);

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
                        let command = Command::new("yarn").current_dir(entry.path()).spawn();
                        println!("Running yarn in {}", program);
                        match command {
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
                    mctl_update(&program);
                }
            }
        }
    }
    let mut all_results = Vec::new();

    // Separate iteration for running tests
    for entry in &entries[..] {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let program = entry.file_name();
                    let result = mctl_test(program.to_str().unwrap());
                    if !result.success {
                        let message = format!("❌ Test failed for {:?}", program);
                        if let Err(err) = send_message(&message).await {
                            let err_message = format!("HTTP Error: {}", err);
                            return Err(Error::from_str(&err_message));
                        }
                    }else {
                        let message = format!("🎉 Test passed for {:?}", program);
                        if let Err(err) = send_message(&message).await {
                            let err_message = format!("HTTP Error: {}", err);
                            return Err(Error::from_str(&err_message));
                        }
                    }
                    all_results.push(result);
                }
            }
        }
    }

    // Save all test results to JSON
    if let Ok(json_data) = serde_json::to_string_pretty(&all_results) {
        let file_path = "all_results.json";
        if let Ok(mut file) = File::create(file_path) {
            if let Err(err) = file.write_all(json_data.as_bytes()) {
                eprintln!("Error writing test results to file: {}", err);
            }
        } else {
            eprintln!("Error creating file for test results");
        }
    } else {
        eprintln!("Error serializing test results to JSON");
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

    let _ = copy_folder(&source_folder, &membrane_dir);

    mctl_update("http");
    Ok(())
}

fn mctl_update(program: &str) {
    let command = Command::new("mctl").arg("update").arg(program).spawn();

    match command {
        Ok(mut child) => {
            println!("Updating {:?}", program);

            let exit_status = child
                .wait()
                .expect("Failed to wait on child process for test");

            if exit_status.success() {
                // successful execution
            } else {
                // unsuccessful execution
                // exit(1);
            }
        }
        Err(e) => {
            println!("Failed to execute command: {}", e);
            exit(1);
        }
    }
}

fn mctl_test(program: &str) -> TestResult {
    let command = Command::new("mctl").arg("test").arg(program).spawn();

    match command {
        Ok(mut child) => {
            println!("Running test in {:?}", program);

            let exit_status = child
                .wait()
                .expect("Failed to wait on child process for test");

            let success = exit_status.success();
            let test_result = TestResult {
                program: program.to_string(),
                success,
            };

            test_result
        }
        Err(e) => {
            println!("Failed to execute command: {}", e);
            exit(1);
        }
    }
}

fn copy_folder(source: &Path, destination: &Path) -> io::Result<()> {
    if !destination.exists() {
        fs::create_dir_all(destination)?;
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();

        let destination_path = destination.join(file_name);

        if entry_path.is_dir() {
            copy_folder(&entry_path, &destination_path)?;
        } else {
            fs::copy(&entry_path, &destination_path)?;
        }
    }

    Ok(())
}
async fn send_message(message: &str) -> Result<(), HttpError> {
    let client = Client::new();
    // TODO: replace with webhook URL
    let url = "https://discord.com/api/webhooks/1128740471552880640/tExzZQ1LmRDWnP_qrTDVgBIzwZ5ewpMmQYL8FcU6OY6tuB74HMe5BV5mXVAJ7oEIMdMY";

    let params = json!({
        "username": "Test Bot",
        "content": message,
    });

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&params)
        .send()
        .await?;

    let status_code = response.status().as_u16();
    if status_code > 299 {
        println!("Error sending message: {:?}", response);
    } else {
        println!("Message sent successfully");
    }
    Ok(())
}
