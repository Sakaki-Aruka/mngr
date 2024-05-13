use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use std::fmt::format;
use std::fs::File;
use std::io::{Result, stdin, Write};
use std::ops::Add;
use chrono::{DateTime, FixedOffset, ParseResult, Utc};
use colored::{ColoredString, Colorize};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use toml::macros::push_toml;

#[derive(Debug, Serialize, Deserialize)]
struct AppData {
    id: String,
    created_date: String,
    github_token: String,
    plugins: HashMap<String, PluginData>,
}

impl AppData {
    pub fn new(id: Option<String>, date: Option<String>, token: Option<String>) -> Self {
        AppData {
            id: if id.is_some() { id.unwrap() } else { format!("{}", Uuid::new_v4()) },
            created_date: if date.is_some() { date.unwrap() } else { Utc::now().to_string() },
            github_token: if token.is_some() { token.unwrap() } else { String::new() },
            plugins: HashMap::new(),
        }
    }

    pub fn get_created_utc(&self) -> Option<DateTime<Utc>> {
        let parse_result: ParseResult<DateTime<FixedOffset>> = DateTime::parse_from_rfc3339(self.created_date.as_str());
        if parse_result.is_err() { return None };
        Some(parse_result.unwrap().to_utc())
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct PluginData {
    name: String,
    version: String,
    introduced_date: String,
    description: Option<Vec<String>>,
}

impl PluginData {
    pub fn new(name: String, version: String, date: DateTime<Utc>,description: Option<Vec<String>>) -> Self {
        PluginData {
            name,
            version,
            introduced_date: date.to_string(),
            description: if description.is_some() { description } else { None },
        }
    }

    pub fn content(&self) -> String {
        let mut content: String = String::new();
        content.push_str(format!("- name: {}", self.name).as_str());
        content.push_str(format!("- version: {}", self.version).as_str());
        content.push_str(format!("- introduced date: {}", self.introduced_date.to_string()).as_str());
        content
    }
}

fn main() {
    let config_path: Option<PathBuf> = get_config_path();
    if config_path.is_none() {
        println!("{}", "Failed to get path.".red());
        return;
    }
    let app: Option<AppData> =
        if !config_path.unwrap().exists() {
            create_config()
        } else {
            get_config()
        };

    if app.is_none() {
        println!("{}", "Failed to create 'mngr.toml'. Process closed.".red());
        return;
    }

    let app: AppData = app.unwrap();

    loop {
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        input.trim().to_string();
        let input: &str = input.trim_matches(|c| c == '\r' || c == '\n');
        match input {
            "exit" => break,
            "help" => show_help(),
            _ => (),
        }

        println!("> {}", &input);
    }

    //debug
    print_plugins(&app);
    dbg!(app);
}

fn show_help() {
    println!("'{}' or '{}' - {}", "help".green(), "H".green(), "show this page.");
    println!("'{}' or '{}' - {}", "register (repository url)".green(), "R (repository url)".green(), "register a specified plugin repository.");
    println!("'{}' or '{}' - {}", "unregister (plugin name)".green(), "UR (plugin name)".green(), "unregister a specified plugin from mngr.");
    println!("'{}' or '{}' - {}", "update (plugin_name or 'all')".green(), "U (plugin_name or 'all')".green(), "update a specified or all plugins.");
}

fn print_plugins(app: &AppData) {
    let end: ColoredString = "End of the plugins list.".green();
    if app.plugins.is_empty() {
        println!("{}", &end);
        return;
    }
    for plugin in &app.plugins {
        println!("{}", plugin.1.content());
    }
    println!("{}", &end);
}

fn create_config() -> Option<AppData> {
    let current: Result<PathBuf> = env::current_dir();
    if current.is_err() {
        println!("{}", "Failed to get current directory.".red());
        return None
    };
    let mut current: PathBuf = current.unwrap();
    current.push("mngr.toml");
    let file: Result<File> = File::create_new(current.as_path());
    if file.is_err() {
        println!("{}", "Failed to create 'mngr.toml'. It has already exists.".red());
        return None
    };
    let mut file: File = file.unwrap();

    let id: String = format!("{}", Uuid::new_v4());
    let date: String = Utc::now().to_string();
    let app: AppData = AppData::new(Some(id), Some(date), None);
    write!(file, "{}", toml::to_string(&app).unwrap()).unwrap();
    file.flush().unwrap();

    println!("{}", "Task successful. mngr made 'mngr.toml'.".green());
    Some(app)
}

fn get_config() -> Option<AppData> {
    let element: String = fs::read_to_string("mngr.toml").unwrap();
    let app: core::result::Result<AppData, toml::de::Error> = toml::from_str(element.as_str());
    if app.is_ok() { Some(app.unwrap()) }
    else {
        println!("{}", "Failed to parse elements what are written in 'mngr.toml'.".red());
        None
    }
}

fn get_config_path() -> Option<PathBuf> {
    // checks toml format and contents.
    let path = env::current_dir();
    if path.is_err() {
        println!("{}", "failed to get current directory. try again after sometime.".red());
        return None;
    }
    let mut path: PathBuf = env::current_dir().unwrap();
    path.push("mngr.toml");
    Some(path)
}

