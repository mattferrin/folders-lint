use glob::{MatchOptions, Pattern};
use neon::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct ConfigFile {
    root: String,
    rules: Vec<String>,
}

/**
 * Simplifies tests by displaying errors uniformly.
 * Also using to convert path for regex matching.
 */
fn to_unix_string(name: String) -> String {
    name.replace("\\", "/")
}

fn enforce_config(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let config_file = match cx.argument_opt(0) {
        None => ".folderslintrc.json".to_owned(),
        Some(arg) => arg.downcast::<JsString>().or_throw(&mut cx)?.value(),
    };
    let config_file_display = to_unix_string(config_file.clone());

    let file = File::open(config_file).expect(&format!(
        "Could not open the config file `{}`",
        config_file_display
    ));
    let reader = BufReader::new(file);
    let config_values: ConfigFile = serde_json::from_reader(reader).expect(&format!(
        "Could not parse the `{}` file",
        config_file_display
    ));

    if !Path::new(&config_values.root).exists() {
        panic!(format!(
            "The `root` path is invalid in the config file `{}`",
            config_file_display
        ));
    }
    if config_values.rules.len() < 1 {
        panic!(format!(
            "The `rules` array is empty in the config file `{}`",
            config_file_display
        ));
    }

    let root = config_values.root.clone();
    let root_1 = &root.clone();
    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(move |e| e.ok())
        .filter(|e| !e.file_type().is_dir())
    {
        let entry_1 = entry.clone();
        let satisfies_rule = config_values.rules.clone().into_iter().any(move |rule| {
            let path = format!("{}", entry.path().display());

            match rule.chars().next() {
                // regex rule
                Some('/') => {
                    let root_slash = format!("{}{}", root_1, "/");
                    let unix_path = to_unix_string(path);
                    let relative_path = unix_path.trim_start_matches(&root_slash);
                    let regex_rule = rule.trim_start_matches("/").trim_end_matches("/");
                    let regex = Regex::new(&regex_rule);
                    match regex {
                        Ok(ok) => ok.is_match(relative_path),
                        Err(_) => panic!(format!("Invalid regex rule `{}`", rule)),
                    }
                }
                // glob rule
                Some(_) => {
                    let extended_rule = format!("{}/{}", root_1, rule);

                    let is_satisfied = Pattern::new(&extended_rule).unwrap().matches_with(
                        &to_unix_string(path),
                        MatchOptions {
                            case_sensitive: true,
                            require_literal_leading_dot: false,
                            require_literal_separator: true, // because default false does not match standard glob behavior
                        },
                    );
                    is_satisfied
                }
                None => {
                    panic!("A rule cannot be empty")
                }
            }
        });
        if !satisfies_rule {
            panic!(format!(
                "The following file path does not satisfy any rules `{}`",
                to_unix_string(format!("{}", entry_1.path().display()))
            ));
        }
    }
    println!("No rule violations detected");
    Ok(cx.undefined())
}

register_module!(mut cx, {
    cx.export_function("enforceConfig", enforce_config)
});
