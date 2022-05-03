use std::{
    fs::{self, File},
    io::{Write},
    path::Path,
};
use crate::{auth::Auth, util::*};
use anyhow::Ok;
use anyhow::Result;
use serde::{Deserialize, Serialize};
/// Global and local configuration.
/// This module manages the configuration files for polymc-rs.
use serde_json::{json, Value};
// TODO: Actually use this.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// The default game profile.
    pub default_profile: String,
    /// The default user profile.
    pub default_user_profile: String,
}

impl GlobalConfig {

    /// The global configuration file.
    /// This will be located at {main_dir()}/config.json, most of the time.
    pub fn new() -> Self {
        GlobalConfig {
            default_profile: "default".to_string(),
            default_user_profile: "".to_string(),
        }
    }

    /// Load the global configuration file.
    pub fn load() -> Self {
        // TODO: Make this configurable with an env var.
        let path = Path::new(&main_dir()).join("config.json");

        if !path.exists() {
            let mut file = File::create(&path).unwrap();
            serde_json::to_writer_pretty(&mut file, &GlobalConfig::new()).unwrap();
        }

        // then load it.
        let file = File::open(&path).unwrap();
        let config: GlobalConfig = serde_json::from_reader(file).unwrap();
        config
    }

    /// Save the global configuration file.
    pub fn save(&self) -> Result<()> {
        let path = Path::new(&main_dir()).join("config.json");
        let mut file = File::create(&path).expect("Could not create config file");
        serde_json::to_writer_pretty(&mut file, self).expect("Could not write config file");

        Ok(())
    }

}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthProfile {
    /// Name of the profile. This will usually be the username.
    pub name: String,
    /// The Authentication object.
    pub auth: Auth,
}

impl AuthProfile {
    pub fn new(name: &str, auth: Auth) -> Self {
        AuthProfile {
            name: name.to_owned(),
            auth,
        }
    }
    /// Reads the configuration file from a path, and then returns it into an AuthProfile.
    pub fn read_from_file(path: &str) -> Self {

        // Read the file into string
        // contents of the file as a string
        let file = fs::read_to_string(path).unwrap();

        // Parse the string as JSON
        let json: Value = serde_json::from_str(&file).unwrap();

        let profile_name = json["name"].as_str().unwrap();
        // Get the "auth" key's value
        let auth = json["auth"].clone();
        println!("{:#?}", auth["auth_type"].as_str());
        match auth["auth_type"].as_str() {
            Some("offline") => {
                let username = auth["username"].as_str().unwrap();
                let auth = Auth::new_offline(username);
                AuthProfile::new(profile_name, auth)
            }

            Some("mojang") => {
                let username = auth["username"].as_str().unwrap();
                let token = auth["token"].as_str().unwrap();
                let id = auth["uuid"].as_str().unwrap();
                let auth = Auth::Mojang {
                    auth_type: "mojang".to_string(),
                    username: username.to_string(),
                    token: token.to_string(),
                    uuid: id.to_string(),
                };
                AuthProfile::new(profile_name, auth)
            }

            Some("microsoft") => {
                let username = auth["username"].as_str().unwrap();
                let token = auth["token"].as_str().unwrap();
                let id = auth["uuid"].as_str().unwrap();
                let refresh_token = auth["refresh_token"].as_str().unwrap_or("");
                // TODO: Also return a refresh Token
                let auth = Auth::MSFT {
                    auth_type: "microsoft".to_string(),
                    username: username.to_string(),
                    token: token.to_string(),
                    uuid: id.to_string(),
                    refresh_token: refresh_token.to_string(),
                };
                AuthProfile::new(profile_name, auth)
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
        });

        // Write the data to the file (pretty printed)
        serde_json::to_writer_pretty(&mut file, &data).unwrap();
    }

    pub fn load_profile(name: &str) -> Self {
        let path = Path::new(&get_dir("profiles")).join(format!("{}.json", name));
        // Check if the file exists and is valid
        if !path.exists() {
            AuthProfile::new(name, Auth::new_offline(name))
        } else {
            AuthProfile::read_from_file(&path.to_str().unwrap())
        }
    }
}



// le command line

