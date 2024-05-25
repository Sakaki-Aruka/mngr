use std::collections::HashMap;
use std::path::{PathBuf};
use std::{env, fs};
use std::arch::x86_64::_mm_add_pd;
use std::fs::{File};
use std::io::{Result, stdin, stdout, Write};
use std::ops::DerefMut;
use std::str::{Bytes, FromStr};
use chrono::{DateTime, FixedOffset, ParseResult, TimeDelta, Utc};
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
    repository_url: String,
    latest_in_the_time: bool
}

impl PluginData {
    pub fn new(name: String, version: String, date: DateTime<Utc>,description: Option<Vec<String>>, pre_release: bool, file_name: String, repository_url: String, is_latest: bool) -> Self {
        PluginData {
            name,
            version,
            introduced_date: date.to_string(),
            description: if description.is_some() { description } else { None },
            pre_release,
            file_name,
            repository_url,
            latest_in_the_time: is_latest,
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
            repository_url: String::new(),
            latest_in_the_time: true,
        }
    }

    pub fn content(&self) -> String {
        let mut content: String = String::new();
        content.push('\n');
        content.push_str(format!("- name: {}\n", self.name).as_str());
        content.push_str(format!("- version: {}\n", self.version).as_str());
        content.push_str(format!("- introduced date: {}\n", self.introduced_date.to_string()).as_str());
        content.push_str(format!("- pre release: {}\n", self.pre_release.to_string()).as_str());
        content.push_str(format!("- filename: {}\n", self.file_name.as_str()).as_str());
        content.push_str(format!("- repository url: {}", self.repository_url.as_str()).as_str());
        content.push('\n');
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
            // write here (have to displays the result of a process. 'succeeded' or 'failed'.)
            "exit" | "E" | "e" => break,
            "help" | "H" | "h" => show_help(),
            "register" | "R" | "r" => register_listener(&mut app),
            "unregister" | "UR" | "ur" => unregister_listener(&mut app),
            "list" | "L" | "l" => print_plugins(&app),
            "update" | "U" | "u" => update_listener(&mut app),
            _ => {
                println!("{}", "Enter 'help' or 'H', displayed command helps.".underline());
            },
        }
        print!("mngr > ");
        stdout().flush().unwrap();
    }
    config_update(&app);
}

fn config_update(app: &AppData) {
    let path: Option<PathBuf> = get_config_path();
    if path.is_none() {
        println!("{}", "Failed to get 'mngr.toml' path.".red());
        println!("{}", "mngr will not save the data.".yellow());
        return;
    }
    let path: PathBuf = path.unwrap();
    let mut file: Result<File> = File::create(path.as_path());
    if file.is_err() {
        println!("{}", "Failed to handle 'mngr.toml'.".red());
        println!("{}", "mngr will not save the modified data.".yellow());
        return;
    }
    let mut file: File = file.unwrap();
    let content: String = toml::to_string(app).unwrap();
    write!(file, "{}", content).unwrap();
    file.flush().unwrap();
}

fn unregister_listener(app: &mut AppData) {
    let help_1: ColoredString = ColoredString::from("Enter 'exit' or 'e', leave unregister mode.").yellow();
    let help_2: ColoredString = ColoredString::from("Enter 'help' or 'h', display these helps.").yellow();
    'main:loop {
        print!("mngr > {}register > ", "un".underline());
        stdout().flush().unwrap();
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        let input: String = input.trim_end().to_string();
        let mut args: Vec<String> = input.split(" ").map(|c| String::from(c)).collect();
        // plugin_name or file_name -> '-n (plugin name)' or '-f (plugin file's name)', and default '(plugin name)' works like '-n (plugin name)'
        if args.len() < 1 || args.len() > 2 || args[0].is_empty() {
            println!("{}", "Invalid arguments. It needs only 1 or 2 arguments.".red());
            println!("{}", "'-n (plugin name)', '-f (plugin file's name)' or '(plugin name)'.".yellow());
            println!("{}", &help_1);
            println!("{}", &help_2);
            continue
        }
        if args.len() == 1 {
            match args[0].as_str() {
                "exit" | "e" => break 'main,
                "help" | "h" => {
                    println!("{}", &help_1);
                    println!("{}", &help_2);
                    continue
                },
                _ => (),
            }
        }
        if args.len() == 1 { args.insert(0, String::from("-n")) };
        let removed: Option<String>;
        if args.get(0).unwrap().as_str() == "-n" {
            let result: Option<PluginData> = app.plugins.remove(&args[1]);
            removed = if result.is_some() { Some(result.unwrap().file_name) } else { None };
        } else if args.get(0).unwrap().as_str() == "-f" {
            let size: usize = app.plugins.len();
            app.plugins.retain(|k, v| v.file_name != args[1]);
            removed = if app.plugins.len() != size { Some(String::from(&args[1])) } else { None };
        } else {
            println!("{}", &help_1);
            println!("{}", &help_2);
            continue 'main;
        }

        if removed.is_none() {
            println!("{}{} {}{}", "Failed to unregister. (".red(), if args[0].as_str() == "-n" { "FileName:" } else { "PluginName:" }, &args[1], ")".red());
            continue
        }
        let removed: String = removed.unwrap();

        if delete_plugin_jar(&removed, true) {
            println!("{}", "The plugin has been successfully unregistered.".green());
            println!("{} {}", "Removed:".green(), &removed);
        } else { continue }
    }
}

fn delete_plugin_jar(filename: &String, is_unregister: bool) -> bool {
    let plugins_directory: Option<PathBuf> = get_plugins_directory_path();
    if plugins_directory.is_none() {
        println!("{}", "Failed to get 'plugins' directory's path.".red());
        println!("{}", "Change the current directory or make a directory that named 'plugins' here and retry it.".red());
        return false
    }
    let directory: PathBuf = plugins_directory.unwrap();
    let mut file_path: PathBuf = PathBuf::from(directory);
    file_path.push(filename);
    if fs::remove_file(file_path.to_str().unwrap()).is_err() {
        println!("{} -> {}", "Failed to delete the file.".red(), file_path.to_str().unwrap());
        if is_unregister { println!("{}", "* The specified plugin has already unregistered from mngr.".yellow()); };
        return false
    }
    true
}

fn register_listener(app: &mut AppData) {
    let help_1: ColoredString = ColoredString::from("Enter 'exit' or 'e', leave register mode.").yellow();
    let help_2: ColoredString = ColoredString::from("Enter 'help' or 'h', display these helps.").yellow();
    loop {
        print!("mngr > register > ");
        stdout().flush().unwrap();
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        let input: String = input.trim_end().to_string();
        let args: Vec<String> = input.split(" ").map(|c| String::from(c)).collect();
        if args.len() != 1 {
            println!("{}{}", "Failed to parse arguments. It needs only 1 arg. -> ".red(), "'GitHub repository URL'".yellow());
        } else if args.len() == 1 {
            match args[0].as_str() {
                "exit" | "e" => {
                    break
                },
                "help" | "h" | "" => {
                    if args[0].as_str() == "" {
                        // when empty
                        println!("{}", "Invalid arguments. It needs only one argument.".red());
                        println!("{}", "-> '(repository url)' (e.g. 'https://github.com/Sakaki-Aruka/custom-crafter')".yellow());
                    }
                    println!("{}", &help_1);
                    println!("{}", &help_2);
                    continue
                },
                // 1:https://github.com/Sakaki-Aruka/custom-crafter -> latest -1 version register and unregister same name plugin. (when exist)
                _ => (),
            }
        } else if args.is_empty() {
            println!("{}", &help_1);
            println!("{}", &help_2);
            continue
        }
        if !register(app, &args[0]) {
            println!("{}", "Failed to register.".red());
        }
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
            if &response.status().as_u16() == &401 {
                println!("{}{}", "\n", "Detected 401 error.".yellow());
                println!("{}", "This error means that you sent an incorrect authorization token with the request.".yellow().underline());
                println!("{}", "You have to check your github api token what written in 'mngr.toml' and those expiration.".yellow().underline());
            }
            return false;
        }
    }
    let api_remaining: Option<i16> = get_rate_limit_remaining(&response);
    let response_result: Option<PluginData> = get_latest_plugin(&mut response_parser(response));
    if response_result.is_none() {
        println!("{}", "Failed to get plugin data.");
        return false
    }
    let plugin: PluginData = response_result.unwrap();
    let name: String = String::from(&plugin.name);
    let plugin_info: String = plugin.content();
    if app.plugins.contains_key(&name) {
        println!("{}", "The plugin has already registered.".yellow());
        let registered: &PluginData = app.plugins.get(&name).unwrap();
        println!("{}", registered.content());
        return false
    }
    app.plugins.insert(name, plugin);

    let api_remaining: String =
        if api_remaining.is_some() { api_remaining.unwrap().to_string() }
        else { String::from("UNKNOWN") };
    println!("{}", "The plugin has been successfully registered.".green());
    println!("{}", plugin_info);
    println!("API CALL REMAINING: {}", api_remaining);
    true
}

fn get_plugins_directory_path() -> Option<PathBuf> {
    let mut current: PathBuf = match env::current_dir() {
        Ok(path) => path,
        _ => return None,
    };
    current.push("plugins");
    return Some(current)
}


fn update_listener(app: &mut AppData) {
    // input types
    // (implemented) #all -> all update to latest version
    // (implemented) #!pre -> update to latest "stable" release
    // (no used) #only-marked -> updates what is marked 'latest'
    // (no used) #!only-marked -> updates what is not marked 'latest'
    // #pls (plugin_name,plugin_name,plugin_name) -> updates what is specified
    // #~(RFC3339 formatted date) -> updates what is published before specified date
    // #(RFC3339 formatted date)~ -> updates what is published after specified date
    // #plv -> Enter select plugin and version mode.
    //
    loop {
        print!("mngr > update > ");
        stdout().flush().unwrap();
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        let input: String = input.trim_end().to_string();
        match input.as_str() {
            "exit" | "E" | "e" => break,
            "#all" => {
                let all: Vec<String> = app.plugins.keys().map(|k: &String| String::from(k)).collect();
                all_update(&all, app);
            }
            "#!pre" => {
                let without_pre: Option<Vec<String>> = get_not_prerelease_plugins_name(&app);
                if without_pre.is_none() {
                    println!("{}", "mngr does not have any plugins that are marked as 'pre-release'.".green());
                    continue
                }
                all_update(&without_pre.unwrap(), app);
            },
            "#multi" => {
                multiple_plugins_update_listener(app);
            }
            _ => (),
        }
    }
}

fn get_not_prerelease_plugins_name(app: &AppData) -> Option<Vec<String>> {
    if app.plugins.is_empty() { return None };
    let mut result:  Vec<String> = Vec::new();
    for pl in app.plugins.values() {
        if !pl.pre_release { result.push(String::from(&pl.name)); }
    }
    Some(result)
}

fn multiple_plugins_update_listener(app: &mut AppData) {
    let mut candidate: Vec<String> = Vec::new();
    loop {
        print!("mngr > update > multi > ");
        stdout().flush().unwrap();
        let mut input: String = String::new();
        stdin().read_line(&mut input).ok();
        let input: String = input.trim_end().to_string();
        if input.is_empty() {
            println!("{}", "Enter plugins name those separated with ','.".yellow());
            continue
        }
        for p in input.split(",").map(|c| String::from(c)) {
            candidate.push(p.replace(" ", ""));
        }
        break
    }

    let mut remove_candidate: Vec<String> = Vec::new();
    for plugin in app.plugins.values() {
        if !candidate.contains(&plugin.name) { continue };
        let builder: RequestBuilder = get_releases_request_builder(&plugin, app);
        let response: reqwest::Result<Response> = builder.send();
        if response.is_err() {
            println!("{}", "Failed to get plugin data from GitHub API.".red());
            continue
        }
        let response: Response = response.unwrap();
        let mut plugins: HashMap<DateTime<Utc>, PluginData> = response_parser(response);
        remove_pre_release(&mut plugins);
        remove_candidate.push(String::from(&plugin.name));
    }
    if !remove_candidate.is_empty() {
        all_update(&remove_candidate, app);
    }
}

fn remove_pre_release(data: &mut HashMap<DateTime<Utc>, PluginData>) {
    let mut remove_candidate: Vec<DateTime<Utc>> = Vec::new();
    for content in data.iter().clone() {
        if content.1.pre_release { remove_candidate.push(content.0.clone()); }
    }
    if remove_candidate.is_empty() { return };
    for key in remove_candidate {
        data.remove(&key);
    }
}

fn get_latest_plugin(data: &mut HashMap<DateTime<Utc>, PluginData>) -> Option<PluginData> {
    if data.is_empty() { return None };
    let latest_date: &DateTime<Utc> = &data.keys().copied().max().unwrap();
    data.remove(latest_date)
}

fn get_releases_request_builder(pl: &PluginData, app: &AppData) -> RequestBuilder {
    let url_parsed: Vec<String> = String::from(&pl.repository_url).split("/").map(|c| String::from(c)).collect();
    let request_url: String = String::from(format!("https://api.github.com/repos/{}/{}/releases", &url_parsed[3], &url_parsed[4]));
    let mut builder: RequestBuilder = blocking::Client::new().get(&request_url);
    if !&app.github_token.is_empty() { builder = builder.header("Authorization", format!("token {}", &app.github_token)); };
    builder = builder.header("X-GitHub-Api-Version", "2022-11-28");
    builder = builder.header("User-Agent", "mngr");
    builder = builder.header("Accept", "application/vnd.github.v3+json");
    builder
}
fn all_update(data: &Vec<String>, app: &mut AppData) {
    let mut new: Vec<PluginData> = Vec::new();
    for name in data {
        let pl: &PluginData = app.plugins.get(name).unwrap();
        println!("\nUpdate Target = {}", &pl.name.underline());
        let builder: RequestBuilder = get_releases_request_builder(pl, app);
        let response: reqwest::Result<Response> = builder.send();
        if response.is_err() {
            println!("{}", "Failed to get plugin data from GitHub API.".red());
            continue
        }
        let response: Response = response.unwrap();
        let mut plugins: HashMap<DateTime<Utc>, PluginData> = response_parser(response);
        remove_pre_release(&mut plugins);
        if plugins.is_empty() {
            println!("{} '{}'", "No releases in".red(), &pl.name.underline());
            continue
        }
        let plugin: PluginData = get_latest_plugin(&mut plugins).unwrap();
        if pl.version == plugin.version {
            println!("{}", "The latest version is same with existed.".green());
            println!("{}", "So mngr skips to update.".green());
            continue
        }
        if !delete_plugin_jar(&pl.file_name, false) {
            println!("{}", "Failed to remove the plugin file.".red());
            println!("{} {}\n", "Continued to update".green(), &pl.name.underline());
        }
        if !jar_download(&plugin) {
            println!("{}", "Failed to download the plugin jar file.".red());
            continue
        }
        new.push(plugin);
    }

    if new.is_empty() { return };
    for plugin in new {
        app.plugins.remove(&plugin.name);
        app.plugins.insert(String::from(&plugin.name), plugin);
    }
}


fn jar_download(plugin: &PluginData) -> bool {
    // https://github.com/Sakaki-Aruka/custom-crafter/releases/tag/v4.1.6
    // https://github.com/Sakaki-Aruka/custom-crafter/releases/download/v4.1.6/custom-crafter-4.1.6.jar
    // -> (repository-url)/releases/download/(version)/(file name)
    let download_url: String = String::from(format!("{}/releases/download/{}/{}", &plugin.repository_url.as_str(), &plugin.version, &plugin.file_name)); // fix here
    let mut builder: RequestBuilder = blocking::Client::new().get(&download_url);
    builder = builder.header("User-Agent", "mngr");
    let response: reqwest::Result<Response> = builder.send();
    if response.is_err() {
        println!("{} From: {}", "Failed to download a release file.".red(), &download_url.underline());
        return false
    }
    let response: Response = response.unwrap();

    let filename: String = String::from(&plugin.file_name);
    let path: Option<PathBuf> = get_plugins_directory_path();
    if path.is_none() {
        println!("{}", "Failed to handle 'plugins' directory's path.".red());
        return false;
    }
    let mut path: PathBuf = path.unwrap();
    path.push(filename);
    if path.exists() {
        println!("{}", "The file has already exists. What do you want to do to it?".yellow());
        println!("Target file: {}", &plugin.file_name.underline());
        println!(" {}, or {}", "Delete and Update (Enter '0')".green(), "Not change (Enter other than '0')".yellow());
        print!("mngr > update > select > ");
        stdout().flush().unwrap();
        let mut select: String = String::new();
        stdin().read_line(&mut select).ok();
        let select: String = select.trim_end().to_string();
        match select.as_str() {
            "0" => {
                if fs::remove_file(&path.as_path()).is_err() {
                    println!("{}", "Failed to remove the file.".red());
                    return false
                }
                if fs::File::create(&path.as_path()).is_err() {
                    println!("{}", "Failed to create the file.".red());
                    return false
                }
            },
            _ => {
                println!("{} {}.", "Cancel to install".yellow(), &plugin.file_name.yellow());
                return false
            }
        }
    }
    let content = response.bytes().unwrap();
    if fs::write(&path.as_path(), content).is_err() {
        println!("{}", "Failed to save the downloaded content.".red());
        false
    } else {
        println!("{}", "The plugin has been successfully download.".green());
        println!("{} '{}'", "Saved as", &path.to_str().unwrap());
        true
    }
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


fn response_parser (response: Response) -> HashMap<DateTime<Utc>, PluginData> {
    // json parser -> https://docs.rs/serde_json/latest/serde_json/
    let response_str: reqwest::Result<String> = response.text();
    if response_str.is_err() {
        println!("{}", "Failed to receive an API response.".red());
        return HashMap::new()
    };
    // hashmap -> key: plugin name, value: PluginData
    let response_str: String = response_str.unwrap();
    let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(response_str.as_str());
    if parsed.is_err() {
        println!("{}", "Mapping failed to PluginData from the response data.".red());
        return HashMap::new()
    }

    let parsed: Value = parsed.unwrap();

    let mut unsorted_data: HashMap<DateTime<Utc>, PluginData> = HashMap::new();

    for i in parsed.as_array() {
        for j in i.iter() {
            let some_base: Vec<String> = j["html_url"].as_str().unwrap().split("/").map(|c| String::from(c)).collect();
            let name: String = String::from(some_base.get(4).unwrap().as_str());
            let version: String = String::from(some_base.get(7).unwrap().as_str());
            let pre_release: bool = j["prerelease"].as_bool().unwrap();
            let repository_url_base: Vec<String> = j["html_url"].to_string().split("/").map(|c| String::from(c)).collect();
            let repository_url: String = format!("https://github.com/{}/{}", &repository_url_base[3], &repository_url_base[4]);
            let mut file_name: String = String::new();
            let mut created_date: String = String::new();
            if j["assets"].as_array().unwrap().is_empty() { continue };
            for k in j["assets"].as_array() {
                file_name.push_str(k[0]["name"].as_str().unwrap());
                created_date.push_str(k[0]["created_at"].as_str().unwrap());
            }
            let description: Option<Vec<String>> = if j["body"].as_str().is_some() { Some(vec![String::from(j["body"].as_str().unwrap().replace("\r\n", "\n"))]) } else { None };
            let date: DateTime<Utc> = DateTime::parse_from_rfc3339(&created_date).unwrap().to_utc();
            let key: DateTime<Utc> = date;
            let plugin: PluginData = PluginData::new(name, version, date, description, pre_release, file_name, repository_url, true);

            unsorted_data.insert(key, plugin);
        }
    }
    unsorted_data
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
    println!("'{}' or '{}' - {}", "unregister".green(), "UR".green(), "Enter 'unregister' mode.");
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

