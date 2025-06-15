use std::path::PathBuf;

use clap::{Arg, Command, value_parser};
pub struct PodCli {
    pub command: Vec<String>,
}

impl PodCli {
    pub fn new() -> Self {
        let matches = Command::new("kube-exec-rs")
            .version("v0.0.1")
            .arg(
                Arg::new("text")
                    .short('s')
                    .help("script text to run, either text or file be set"),
            )
            .arg(
                Arg::new("file")
                    .short('f')
                    .value_parser(value_parser!(PathBuf))
                    .help("script file to run, either text or file be set"),
            )
            .get_matches();

        let mut command = Vec::new();
        if let Some(text_arg) = matches.get_one::<String>("text") {
            command.push(text_arg.to_string());
        };
        if let Some(file_arg) = matches.get_one::<PathBuf>("file") {
            let file = std::fs::read(file_arg);
            match file {
                Ok(file) => {
                    command.push(String::from_utf8_lossy(&file).to_string());
                }
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            }
        };
        if command.is_empty() {
            eprintln!("Error: No script provided");
            std::process::exit(1);
        }

        Self { command: command }
    }
}
