use anyhow::anyhow;
use async_recursion::async_recursion;
use git2::Repository;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Write;
use std::{
    env, fs,
    fs::File,
    io,
    path::Path,
    process::{exit, Command},
};

#[derive(Debug)]
struct Program {
    name: String,
    dependencies: Vec<String>,
    npm_dependencies: bool,
}

#[derive(Serialize)]
struct TestResult {
    program: String,
    success: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Clone a Membrane directory
    let url = "https://github.com/membrane-io/directory.git";

    let repo = match Repository::clone(url, "directory") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };
    // Update all submodules
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

    // Copy the directory to the home directory
    let _ = copy_folder(&source_folder, &membrane_dir);
    println!("Folder copied successfully!");

    // create a array of all the folders in the directory
    // skip the .git folder
    let mut entries: Vec<_> = match fs::read_dir(&membrane_dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("Error reading directory: {}", err);
            std::process::exit(1);
        }
    }
    .filter_map(Result::ok)
    .filter(|entry| {
        let path = entry.path();
        entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
            && path.file_name().map(|name| name != ".git").unwrap_or(false)
    })
    .map(|entry| entry.file_name().to_string_lossy().into_owned())
    .collect();

    // Order the entries so that http is first
    entries.sort_by(|a, b| {
        if a == "todo" {
            std::cmp::Ordering::Less
        } else if b == "todo" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    let mut ordered_programs: HashSet<String> = HashSet::new();
    let mut programs: Vec<Program> = Vec::new();

    // Get all the dependencies for each program
    for entry in entries {
        if let Ok(program) = get_dependencies(&entry) {
            programs.push(program);
        }
    }

    let mut results: Vec<TestResult> = Vec::new();
    // Check all the dependencies and install them recursively
    for program in &programs {
        check_dependencies(&programs, program, &mut ordered_programs, &mut results).await;
    }

    // Save all test results to JSON
    if let Ok(json_data) = serde_json::to_string_pretty(&results) {
        let file_path = "results.json";
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

// execute a update command for a program in the membrane directory
// if has a package.json file, use yarn
// then ren the mctl test command for the program
async fn run_program(program: &Program, results: &mut Vec<TestResult>) -> Result<(), anyhow::Error> {
    println!("Running program: {:?}", program.name);
    if program.npm_dependencies {
        yarn_install(&program.name);
    }
    mctl_update(&program.name);
    let result = mctl_test(&program.name);

    results.push(TestResult {
        program: program.name.clone(),
        success: result.success,
    });
    
    if !result.success {
        let message = format!("❌ Test failed for {:?}", program);
        if let Err(err) = send_message(&message).await {
            let err_message = format!("HTTP Error: {}", err);
            return Err(anyhow!(err_message));
        }
    } else {
        let message = format!("🎉 Test passed for {:?}", program);
        if let Err(err) = send_message(&message).await {
            let err_message = format!("HTTP Error: {}", err);
            return Err(anyhow!(err_message));
        }
    }
    Ok(())
}

#[async_recursion]
async fn check_dependencies(
    programs: &[Program],
    program: &Program,
    ordered_programs: &mut HashSet<String>,
    results: &mut Vec<TestResult>,
) {
    if !program.dependencies.is_empty() {
        for dependency_name in &program.dependencies {
            let dependency = programs.iter().find(|p| p.name == *dependency_name);
            if let Some(dependency) = dependency {
                check_dependencies(programs, dependency, ordered_programs, results).await;
            } else {
                println!("Dependency not found: {}", dependency_name);
            }
        }
    }
    if ordered_programs.insert(program.name.clone()) {
        let _ = run_program(&program, results).await;
    }
}

fn yarn_install(program: &str) {
    let path = dirs::home_dir()
        .expect("Failed to get home directory")
        .join("membrane")
        .join(program);

    let command = Command::new("yarn").current_dir(&path).spawn();

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

async fn send_message(message: &str) -> Result<(), anyhow::Error> {
    let client = Client::new();
    // TODO: replace with webhook URL
    let url = discord_url();

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
    }
    Ok(())
}

fn discord_url() -> String {
    match std::env::var("DISCORD_WEBHOOK_URL") {
        Ok(key) => key,
        Err(_r) => {
            eprintln!("DISCORD_WEBHOOK_URL not set");
            "".to_string()
        }
    }
}

fn get_dependencies(program_name: &str) -> Result<Program, std::io::Error> {
    let membrane_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join("membrane");
    let folder_path = Path::new(&membrane_dir).join(program_name);
    let config_path = folder_path.join("memconfig.json");
    let package_json_path = folder_path.join("package.json");

    if let Ok(contents) = fs::read_to_string(&config_path) {
        let dependencies = extract_program_names(&contents);
        let program = Program {
            name: String::from(program_name),
            dependencies,
            npm_dependencies: package_json_path.exists(), // Check if package.json exists
        };
        Ok(program)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to read config file",
        ))
    }
}

fn extract_program_names(json_str: &str) -> Vec<String> {
    let json_value: Value = serde_json::from_str(json_str).unwrap();
    let dependencies = json_value["dependencies"].as_object().unwrap();

    let mut program_names: Vec<String> = Vec::new();

    for (_, value) in dependencies.iter() {
        if let Some(colon_idx) = value.as_str().and_then(|s| s.find(':')) {
            let program_name = value.as_str().unwrap()[..colon_idx].to_owned();

            if !program_name.starts_with("sys-") {
                program_names.push(program_name);
            }
        }
    }

    program_names.sort();
    program_names.dedup();

    program_names
}
