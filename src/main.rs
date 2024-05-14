use std::collections::HashMap;
use std::path::{PathBuf};
use std::{env, fs};
use std::fs::File;
use std::io::{Result, stdin, stdout, Write};
use std::str::FromStr;
use chrono::{DateTime, FixedOffset, ParseResult, Utc};
use colored::{ColoredString, Colorize};
use http::{HeaderName, HeaderValue};
use fancy_regex::Regex;
use reqwest::{blocking};
use uuid::Uuid;
use serde::{Deserialize, Deserializer, Serialize};
use reqwest::blocking::{RequestBuilder, Response};
use serde_json::Value;

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
    file_name: String,
}

impl PluginData {
    pub fn new(name: String, version: String, date: DateTime<Utc>,description: Option<Vec<String>>, pre_release: bool, file_name: String) -> Self {
        PluginData {
            name,
            version,
            introduced_date: date.to_string(),
            description: if description.is_some() { description } else { None },
            pre_release,
            file_name,
        }
    }

    pub fn empty_new() -> Self {
        PluginData {
            name: String::new(),
            version: String::new(),
            introduced_date: String::new(),
            description: None,
            pre_release: false,
            file_name: String::new(),
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

    let mut app: AppData = app.unwrap();
    print!("mngr > ");
    stdout().flush().unwrap();
    loop {
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        let input: &str = input.trim_end();
        match input {
            "exit" | "E" | "e" => break,
            "help" | "H" | "h" => show_help(),
            "register" | "R" | "r" => register_listener(&mut app),
            _ => (),
        }
        print!("mngr > ");
        stdout().flush().unwrap();
    }

    //debug
    print_plugins(&app);
    dbg!(app);
}

fn register_listener(app: &mut AppData) {
    loop {
        print!("mngr > register > ");
        stdout().flush().unwrap();
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        let input: String = input.trim_end().to_string();
        let args: Vec<String> = input.split(" ").map(|c| String::from(c)).collect();
        if args.len() != 1 {
            println!("{}{}", "Failed to parse arguments. It needs only 1 arg. -> ".red(), "'GitHub repository URL'".yellow());
        } else if &args[0] == &"exit" {
            break
        }
        if !register(app, &args[0]) {
            println!("{}", "Failed to register.".red());
        } else { break }
    }
}

fn register(app: &mut AppData, url: &String) -> bool {
    // https://docs.rs/reqwest/latest/reqwest/
    // (API URL) https://api.github.com/repos/(UserName)/(RepositoryName)/releases
    // (NORMAL URL) https://github.com/(UserName)/(RepositoryName) or .git
    let url_pattern: &str = r"^https://github.com/(?=.{0,39}$)(?!.*--)[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?/[\w\.-]+$";
    let url_pattern: Regex = Regex::new(url_pattern).unwrap();
     if !url_pattern.is_match(url.as_str()).unwrap() {
        println!("{}", "Failed to parse the given url.".red());
        return false
    }
    let mut parsed: Vec<String> = Vec::new();
    url.split("/").for_each(|c| parsed.push(String::from(c)));
    let author: String = String::from(&parsed[3]);
    let repository_name: String =
        if parsed[4].ends_with(".git") { parsed[4].replace(".git", "") }
        else { String::from(&parsed[4]) };
    let url: String = format!("https://api.github.com/repos/{}/{}/releases", &author, repository_name);

    let client: blocking::Client = blocking::Client::new();
    let mut builder: RequestBuilder = client.get(&url);
    if !&app.github_token.is_empty() {
        builder = builder.header("Authorization", format!("token {}", &app.github_token));
    }
    builder = builder.header("X-GitHub-Api-Version", "2022-11-28");
    builder = builder.header("User-Agent", "mngr");
    builder = builder.header("Accept", "application/vnd.github.v3+json");
    builder = builder.header("Content-Type", "application/json");

    let response: reqwest::Result<Response> = builder.send();

    if response.is_err() {
        println!("{}", "Failed to send a request or receive a response.".yellow());
        return false
    }
    let response: Response = response.unwrap();

    match &response.status().as_u16() {
        200 => (),
        _ => {
            println!("{} Code: {}", "I received a not correct status code.".yellow(), &response.status().as_u16());
            println!("{}", "Check the destination of the url.".yellow());
            return false;
        }
    }
    let api_remaining: Option<i16> = get_rate_limit_remaining(&response);
    let response_result: Option<(String, PluginData)> = response_parser(response);
    // name, plugin_data
    if response_result.is_none() {
        println!("{}", "Failed to get plugin data.");
        return false
    }
    let response: (String, PluginData) = response_result.unwrap();
    app.plugins.insert(response.0, response.1);

    let api_remaining: String =
        if api_remaining.is_some() { api_remaining.unwrap().to_string() }
        else { String::from("UNKNOWN") };
    println!("API CALL REMAINING: {}", api_remaining);
    true

    // let api_remaining: String =
    //     if api_remaining.is_some() { api_remaining.unwrap().to_string() }
    //     else { String::from("UNKNOWN") };
    // println!("API CALL REMAINING: {}", api_remaining);
    //
    // let response_result: (String, PluginData, String) = response_result.unwrap();
    // let name: String = response_result.0;
    // let plugin: PluginData = response_result.1;
    // let download_link: String = response_result.2;
    // let file_name: String = download_link.split("/").last().unwrap().to_string();
    // let plugins_directory: Option<PathBuf> = get_plugins_directory_path();
    // if plugins_directory.is_none() {
    //     println!("{}", "Failed to get 'plugins' directory's path. Check and retry.".red());
    //     return false
    // }
    // let plugins_directory: PathBuf = plugins_directory.unwrap();
    // if !jar_download(&download_link, &plugins_directory, &file_name) {
    //     println!("{} {}", "Failed to download".red(), &file_name.underline());
    //     return false
    // }
    // // set the filename to the plugin_data
    // // register to the AppData
    // // displays: rate-limit-remaining
    // false
}

fn get_plugins_directory_path() -> Option<PathBuf> {
    //

    None
}

fn jar_download(url: &String, directory: &PathBuf, file_name: &String) -> bool {
    let mut builder: RequestBuilder = blocking::Client::new().get(url);
    builder = builder.header("Content-Disposition", format!("attachment; filename={}", file_name));
    // Content-Type: application/octet-stream
    builder = builder.header("Content-Type", "application/octet-stream");
    let response: reqwest::Result<Response> = builder.send();
    if response.is_err() {
        println!("{}", "Failed to receive API response.".red());
        return false
    }
    let mut directory: PathBuf = PathBuf::from(directory);
    directory.push(PathBuf::from(file_name));
    //
    false
}

fn get_rate_limit_remaining(response: &Response) -> Option<i16> {
    let key: HeaderName = HeaderName::from_str("X-RateLimit-Remaining").unwrap();
    let result: Option<&HeaderValue> = response.headers().get(key);
    if result.is_none() { return None };
    let header_value: &HeaderValue = result.unwrap();
    if header_value.to_str().is_err() { return None };
    if header_value.to_str().unwrap().parse::<i16>().is_ok() {
        Some(header_value.to_str().unwrap().parse().unwrap())
    } else {
        None
    }
}

fn response_parser(response: Response) -> Option<(String, PluginData)> {
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

    let mut unsorted_data: HashMap<DateTime<Utc>, PluginData> = HashMap::new();

    for i in parsed.as_array() {
        for j in i.iter() {
            let some_base: Vec<String> = j["html_url"].as_str().unwrap().split("/").map(|c| String::from(c)).collect();
            let name: String = String::from(some_base.get(4).unwrap().as_str());
            let version: String = String::from(some_base.get(7).unwrap().as_str());
            let pre_release: bool = j["prerelease"].as_bool().unwrap();
            let mut file_name: String = String::new();
            let mut created_date: String = String::new();
            for k in j["assets"].as_array() {
                file_name.push_str(k[0]["name"].as_str().unwrap());
                created_date.push_str(k[0]["created_at"].as_str().unwrap());
            }
            let description: Option<Vec<String>> = if j["body"].as_str().is_some() { Some(vec![String::from(j["body"].as_str().unwrap())]) } else { None };
            let date: DateTime<Utc> = DateTime::parse_from_rfc3339(&created_date).unwrap().to_utc();
            let key: DateTime<Utc> = date;
            let plugin: PluginData = PluginData::new(name, version, date, description, pre_release, file_name);

            unsorted_data.insert(key, plugin);
        }
    }

    let latest_date: Option<DateTime<Utc>> = get_latest_date(&unsorted_data);
    if latest_date.is_none() {
        println!("{}", "Failed to search latest release.".red());
        return None
    }
    let latest_date: DateTime<Utc> = latest_date.unwrap();
    let plugin: PluginData = unsorted_data.remove(&latest_date).unwrap();

    //debug
    dbg!(&plugin);
    Some((String::from(&plugin.name), plugin))

    // if url_list.is_none() {
    //     println!("{}", "Failed to parse the response data.".red());
    //     return None
    // }
    // let url_list: &Vec<Value> = url_list.unwrap();
    //
    // let mut release_date: HashMap<DateTime<Utc>, PluginData> = HashMap::new();
    // let mut download_link: HashMap<DateTime<Utc>, String> = HashMap::new();
    // for v in url_list {
    //     let mut plugin: PluginData = PluginData::empty_new();
    //     plugin.name = String::from(v["url"].to_string().split("/").collect::<Vec<&str>>()[5]);
    //     plugin.version = v["tag_name"].to_string();//String::from(v["tag_name"].as_str().unwrap());
    //     plugin.pre_release = v["prerelease"].as_bool().unwrap();
    //     let date: &str = v["created_at"].as_str().unwrap();
    //     plugin.introduced_date = String::from(date);
    //     let date: DateTime<Utc> = DateTime::from(DateTime::parse_from_rfc3339(date).unwrap());
    //     release_date.insert(date, plugin);
    //     download_link.insert(date, v["assets"].as_array().unwrap()[0]["browser_download_url"].to_string());
    // }

    // let latest_date: Option<DateTime<Utc>> = get_latest_date(&release_date);
    // if latest_date.is_none() {
    //     println!("{}", "Failed to get latest date.".red());
    //     return None
    // };
    // let latest_date: DateTime<Utc> = latest_date.unwrap();
    //
    // let plugin: PluginData = release_date.remove(&latest_date).unwrap();
    // let download_link: String = download_link.remove(&latest_date).unwrap();
    // let name: String = String::from(&plugin.name);
    // Some((name, plugin, download_link))
}

fn get_latest_date(map: &HashMap<DateTime<Utc>, PluginData>) -> Option<DateTime<Utc>> {
    let mut result: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
    for v in map.keys() {
        if v.signed_duration_since(result).num_seconds() > 0 {
            result = *v;
        }
    }
    if result == DateTime::<Utc>::MIN_UTC { None } else { Some(result) }
}

fn print_api_error(cause: Option<String>) {
    print!("{}", "[API ERROR] Failed to complete a task.".red());
    if cause.is_some() { print!("Cause: {}", cause.unwrap()); }
    stdout().flush().unwrap();
}

fn show_help() {
    println!("'{}' or '{}' - {}", "exit".green(), "E".green(), "exit from mngr interface.");
    println!("'{}' or '{}' - {}", "help".green(), "H".green(), "show this page.");
    println!("'{}' or '{}' - {}", "sync".green(), "S".green(), "sync with the plugins directory status.");
    println!("'{}' or '{}' - {}", "register".green(), "R".green(), "Enter 'register' mode.");
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

