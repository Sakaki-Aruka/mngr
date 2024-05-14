use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use std::fs::File;
use std::io::{Result, stdin, stdout, Write};
use std::str::from_boxed_utf8_unchecked;
use chrono::{DateTime, FixedOffset, ParseResult, TimeZone, Utc};
use colored::{ColoredString, Colorize};
use reqwest::{blocking, Client};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use reqwest::blocking::{RequestBuilder, Response};
use serde_json::Value;
use toml::value::Datetime;

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
    pre_release: bool,
}

impl PluginData {
    pub fn new(name: String, version: String, date: DateTime<Utc>,description: Option<Vec<String>>) -> Self {
        PluginData {
            name,
            version,
            introduced_date: date.to_string(),
            description: if description.is_some() { description } else { None },
            pre_release: false
        }
    }

    pub fn empty_new() -> Self {
        PluginData {
            name: String::new(),
            version: String::new(),
            introduced_date: String::new(),
            description: None,
            pre_release: false,
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
    print!("mngr > ");
    stdout().flush().unwrap();
    loop {
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        input.trim().to_string();
        let input: &str = input.trim_matches(|c| c == '\r' || c == '\n');
        match input {
            "exit" | "E" => break,
            "help" | "H" => show_help(),
            _ => {
                println!("Not found {} command. More help, run 'H'.", &input.yellow());
            },
        }
        print!("mngr > ");
        stdout().flush().unwrap();
    }

    //debug
    print_plugins(&app);
    dbg!(app);
}

fn register(app: &AppData, url: &String) -> bool {
    // https://docs.rs/reqwest/latest/reqwest/
    // (API URL) https://api.github.com/repos/(UserName)/(RepositoryName)/releases
    // (NORMAL URL) https://github.com/(UserName)/(RepositoryName) or .git
    let mut parsed: Vec<String> = Vec::new();
    url.split("/").for_each(|c| parsed.push(String::from(c)));
    let repository_name: String =
        if parsed[5].ends_with(".git") { parsed[5].replace(".git", "") }
        else { String::from(&parsed[5]) };
    let url: String = format!("https://api.github.com/repos/{}/{}/releases", parsed[4], repository_name);
    let body: Response = blocking::get(&url).unwrap();
    match body.status().as_u16() {
        200 => {
            //
        },
        _ => {
            println!("{} Code: {}", "I received a not correct status code.".yellow(), &body.status().as_u16());
            println!("{}", "Check the destination of the url.".yellow());
            return false;
        }
    }
    let client: blocking::Client = blocking::Client::new();
    let mut builder: RequestBuilder = client.get(&url);
    if !&app.github_token.is_empty() {
        builder = builder.header("Authorization", format!("token {}", &app.github_token));
    }
    let response: reqwest::Result<Response> = builder.send();
    if response.is_err() {
        println!("{}", "Failed to send a request or receive a response.".yellow());
        return false;
    }

    let response_result: Option<(String, PluginData, String)> = response_parser(response.unwrap());
    // name, plugin_data, download_url
    if response_result.is_none() {
        println!("{}", "Failed to get plugin data.");
    }
    let response_result: (String, PluginData, String) = response_result.unwrap();
    let name: String = response_result.0;
    let plugin: PluginData = response_result.1;
    let download_link: String = response_result.2;

    // register to the AppData
    // displays: rate-limit-remaining
    false
}

fn response_parser(response: Response) -> Option<(String, PluginData, String)> {
    // json parser -> https://docs.rs/serde_json/latest/serde_json/
    let response_str: reqwest::Result<String> = response.text();
    if response_str.is_err() {
        println!("{}", "Failed to receive an API response.".red());
        return None
    };
    // hashmap -> key: plugin name, value: PluginData
    let response_str: String = response_str.unwrap();
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(response_str.as_str());
    if parsed.is_err() {
        println!("{}", "Mapping failed to PluginData from the response data.".red());
        return None
    }
    let parsed: Value = parsed.unwrap();
    let url_list: Option<&Vec<Value>> = parsed["url"].as_array();
    if url_list.is_none() {
        println!("{}", "Failed to parse the response data.".red());
        return None
    }
    let url_list: &Vec<Value> = url_list.unwrap();

    let mut release_date: HashMap<DateTime<Utc>, PluginData> = HashMap::new();
    let mut download_link: HashMap<DateTime<Utc>, String> = HashMap::new();
    for v in url_list {
        let mut plugin: PluginData = PluginData::empty_new();
        plugin.name = String::from(v["url"].to_string().split("/").collect::<Vec<&str>>()[5]);
        plugin.version = v["tag_name"].to_string();//String::from(v["tag_name"].as_str().unwrap());
        plugin.pre_release = v["prerelease"].as_bool().unwrap();
        let date: &str = v["created_at"].as_str().unwrap();
        plugin.introduced_date = String::from(date);
        let date: DateTime<Utc> = DateTime::from(DateTime::parse_from_rfc3339(date).unwrap());
        release_date.insert(date, plugin);
        download_link.insert(date, v["assets"].as_array().unwrap()[0]["browser_download_url"].to_string());
    }

    let latest_date: Option<DateTime<Utc>> = get_latest_date(&release_date);
    if latest_date.is_none() {
        println!("{}", "Failed to get latest date.".red());
        return None
    };
    let latest_date: DateTime<Utc> = latest_date.unwrap();

    let plugin: PluginData = release_date.remove(&latest_date).unwrap();
    let download_link: String = download_link.remove(&latest_date).unwrap();
    let name: String = String::from(&plugin.name);
    Some((name, plugin, download_link))
}

fn get_latest_date(map: &HashMap<DateTime<Utc>, PluginData>) -> Option<DateTime<Utc>> {
    let mut result: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
    for v in map.keys() {
        if v.max(&result) == v {
            result = *v;
        }
    }
    if result == DateTime::<Utc>::MIN_UTC { None } else { Some(result) }
}

fn print_api_error(cause: Option<String>) {
    print!("{}", "[API ERROR] Failed to complete a task.".red());
    if cause.is_some() { print!("Cause: {}", cause.unwrap()); }
}

fn show_help() {
    println!("'{}' or '{}' - {}", "exit".green(), "E".green(), "exit from mngr interface.");
    println!("'{}' or '{}' - {}", "help".green(), "H".green(), "show this page.");
    println!("'{}' or '{}' - {}", "sync".green(), "S".green(), "sync with the plugins directory status.");
    println!("'{}' or '{}' - {}", "register (repository url)".green(), "R (repository url)".green(), "register a specified plugin repository.");
    println!("'{}' or '{}' - {}", "unregister (plugin name)".green(), "UR (plugin name)".green(), "unregister a specified plugin from mngr.");
    println!("'{}' or '{}' - {}", "update (plugin_name or 'all')".green(), "U (plugin_name or 'all')".green(), "update a specified or all plugins.");
    println!("'{}' or '{}' - {}", "list".green(), "L".green(), "displays all plugins info.");
    println!("'{}' - {}", "remaining".green(), "displays remaining GitHub API request.");
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

