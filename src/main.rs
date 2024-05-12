use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use std::fs::File;
use std::io::{Result, Write};
use chrono::{DateTime, FixedOffset, ParseResult, Utc};
use colored::Colorize;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

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
    released: String,
    description: Option<Vec<String>>,
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

    //debug
    dbg!(app);
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

