use std::{
    fs::{self, File},
    io::{Write},
    path::Path,
};

use crate::{auth::Auth, util::*};
use serde::{Deserialize, Serialize};
/// Global and local configuration.
/// This module manages the configuration files for polymc-rs.
use serde_json::{json, Value};
// TODO: Actually use this.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub config_path: String,
}

impl GlobalConfig {
    pub fn new() -> Self {
        let mut config_path = main_dir();
        config_path.push_str("/config.json");
        let mut config = GlobalConfig {
            config_path: config_path,
        };
        config
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthProfile {
    pub name: String,
    pub auth: Auth,
    pub refresh_token: Option<String>,
}

impl AuthProfile {
    pub fn new(name: &str, auth: Auth, refresh_token: Option<String>) -> Self {
        AuthProfile {
            name: name.to_owned(),
            auth,
            refresh_token,
        }
    }

    pub async fn read_from_file(path: &str) -> Self {
        // Read the file into string
        // contents of the file as a string
        let file = fs::read_to_string(path).unwrap();

        // Parse the string as JSON
        let json: Value = serde_json::from_str(&file).unwrap();

        let profile_name = json["name"].as_str().unwrap();
        // Get the "auth" key's value
        let auth = json["auth"].clone();

        match auth["auth"].as_str() {
            Some("offline") => {
                let username = auth["username"].as_str().unwrap();
                let auth = Auth::new_offline(username);
                AuthProfile::new(profile_name, auth, None)
            }

            Some("mojang") => {
                let username = auth["username"].as_str().unwrap();
                let password = auth["password"].as_str().unwrap();
                let auth = Auth::new_mojang(username, password);
                AuthProfile::new(profile_name, auth, None)
            }

            Some("microsoft") => {
                let username = auth["username"].as_str().unwrap();
                let token = auth["token"].as_str().unwrap();
                let id = auth["id"].as_str().unwrap();
                let refresh_token = auth["refresh_token"].as_str().unwrap_or("");
                // TODO: Also return a refresh Token
                let auth = Auth::MSFT {
                    auth_type: "microsoft".to_string(),
                    username: username.to_string(),
                    token: token.to_string(),
                    uuid: id.to_string(),
                    refresh_token: refresh_token.to_string(),
                };
                AuthProfile::new(profile_name, auth, Some(refresh_token.to_string()))
            }

            _ => {
                // Error out: Unsupported auth type
                panic!("Unsupported auth type");
            }
        }
    }

    pub fn write_to_file(&self) {
        let path = get_dir("profiles");

        // Create the directory if it doesn't exist
        let path_obj = Path::new(&path);
        if !path_obj.exists() {
            fs::create_dir_all(&path).unwrap();
        }

        // Make a file called {self.name}.json
        let mut file = File::create(path_obj.join(format!("{}.json", self.name))).unwrap();

        // First, serialize the auth object
        let auth_json = serde_json::to_value(&self.auth).unwrap();

        let data = json!({
            "name": self.name,
            "auth": auth_json,
            "refresh_token": self.refresh_token.as_ref().unwrap_or(&"".to_owned()),
        });

        // Write the data to the file
        file.write_all(data.to_string().as_bytes()).unwrap();
    }
}
