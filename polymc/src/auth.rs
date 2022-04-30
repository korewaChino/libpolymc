use std::borrow::BorrowMut;
use std::env;

use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
// use HTTP for logging in?
use serde_json::{json, Value};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Auth {
    Offline {
        auth_type: String,
        username: String,
    },
    Mojang {
        auth_type: String,
        username: String,
        token: String,
        uuid: String,
    },
    MSFT {
        auth_type: String,
        username: String,
        token: String,
        uuid: String,
        refresh_token: String,
    },
}

fn random_string() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // print the request
    println!("{:?}", _req);
    Ok(Response::new("Hello, World".into()))
}

impl Auth {
    /// Create a new offline user.
    pub fn new_offline(name: &str) -> Self {
        Auth::Offline {
            auth_type: "offline".to_owned(),
            username: name.to_owned(),
        }
    }
    /// Get The username from the current auth scheme.
    pub fn get_username(&self) -> &str {
        match self {
            Auth::Offline { ref username, .. } => username,
            Auth::Mojang { ref username, .. } => username,
            Auth::MSFT { .. } => unimplemented!(), // TODO: Get the username later
        }
    }

    pub fn get_token(&self) -> Option<&str> {
        match self {
            Auth::Offline { .. } => None,
            Auth::Mojang { token, .. } => Some(token),
            Auth::MSFT { token, .. } => Some(token),
        }
    }

    pub fn get_uuid(&self) -> Option<&str> {
        match self {
            Auth::Offline { .. } => None,
            Auth::Mojang { uuid, .. } => Some(uuid),
            Auth::MSFT { uuid, .. } => Some(uuid),
        }
    }

    pub fn get_auth_type(&self) -> &str {
        match self {
            Auth::Offline { .. } => "offline",
            Auth::Mojang { .. } => "mojang",
            Auth::MSFT { .. } => "microsoft",
        }
    }

    // Online logins

    pub fn new_mojang(username: &str, password: &str) -> Self {
        /// Create a new Mojang auth scheme.
        todo!()
    }

    pub async fn new_microsoft(refresh_token: Option<&str>) -> Self {
        // Credits to https://github.com/ALinuxPerson/mcsoft-auth/ for the MSFT auth scheme.
        // TODO: Make this configurable, Currently it is a dotenv
        dotenv::from_filename(".env").ok();
        let client_id = env::var("AZURE_CLIENT_ID").expect("CLIENT_ID is needed");
        let client_secret = env::var("AZURE_CLIENT_SECRET").expect("CLIENT_SECRET is needed");

        let redirect_url = "http://localhost:8080";

        let state = random_string();
        let url = format!("https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope=XboxLive.signin%20offline_access&state={}", client_id, redirect_url, state);

        if let Err(error) = webbrowser::open(&url) {
            println!("error opening browser: {}", error);
            println!("use this link instead:\n{}", url)
        }

        let query = crate::util::fetch_queries(8080).await;

        let client = reqwest::Client::new();

        println!("Fetching access token");
        let access_token = client
            .post("https://login.live.com/oauth20_token.srf")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("code", query.code),
                ("grant_type", "authorization_code".to_string()),
                ("redirect_uri", redirect_url.to_string()),
            ])
            .send()
            .await
            .expect("Failed to send request")
            .json::<Value>()
            .await
            .expect("Failed to get json");

        //println!("{:#?}", access_token);

        let access_token = access_token["access_token"].as_str().unwrap();

        let json = serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });
        println!("Now authenticating with Xbox Live.");

        let auth_with_xbl = client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&json)
            .send()
            .await
            .expect("Failed to send request")
            .json::<Value>()
            .await
            .expect("Failed to get json");

        //println!("{:#?}", auth_with_xbl);
        let uhashes = auth_with_xbl["DisplayClaims"]["xui"].as_array().unwrap();
        // Find an object with a "uhs" key
        let uhashes = uhashes
            .iter()
            .find(|x| x["uhs"].is_string())
            .expect("no xui found");
        let uhashes = uhashes["uhs"].as_str().unwrap();
        //println!("{:#?}", uhashes);
        let (token, user_hash) = (auth_with_xbl["Token"].as_str().unwrap(), uhashes);
        println!("Now getting an Xbox Live Security Token (XSTS).");

        let json = json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        });

        let auth_with_xsts = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&json)
            .send()
            .await
            .expect("Failed to send request")
            .json::<Value>()
            .await
            .expect("Failed to get json");

        let uhashes = auth_with_xsts["DisplayClaims"]["xui"].as_array().unwrap();
        // Find an object with a "uhs" key
        let uhashes = uhashes
            .iter()
            .find(|x| x["uhs"].is_string())
            .expect("no xui found");
        let uhashes = uhashes["uhs"].as_str().unwrap();
        //println!("{:#?}", uhashes);
        let (token, _) = (auth_with_xsts["Token"].as_str().unwrap(), uhashes);

        let json = json!({ "identityToken": format!("XBL3.0 x={};{}", user_hash, token) });
        println!("Now authenticating with Minecraft.");
        let access_token = client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&json)
            .send()
            .await
            .expect("Failed to send request")
            .json::<Value>()
            .await
            .expect("Failed to get json");

        let access_token = access_token["access_token"].as_str().unwrap();

        println!("Checking for game ownership.");

        let store = client
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&access_token)
            .send()
            .await
            .expect("Failed to send request")
            .json::<Value>()
            .await
            .expect("Failed to get json");

        // Get items in the store
        let items = store["items"].as_array().unwrap();
        //println!("{:#?}", items);
        // check if object with a name key called "game_minecraft" exists
        let _game_minecraft = items
            .iter()
            .find(|x| x["name"].is_string())
            .expect("no game_minecraft found");
        // now do the same thing for product_minecraft
        let _product_minecraft = items
            .iter()
            .find(|x| x["name"].is_string())
            .expect("no product_minecraft found");

        println!("Getting game profile.");

        let profile = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(&access_token)
            .send()
            .await
            .expect("Failed to send request")
            .json::<Value>()
            .await
            .expect("Failed to get json");

        println!("{:#?}", profile);

        let username = profile["name"].as_str().unwrap();
        let id = profile["id"].as_str().unwrap();

        Auth::MSFT {
            auth_type: "microsoft".to_string(),
            username: username.to_string(),
            token: access_token.to_string(),
            uuid: id.to_string(),
            // TODO: Also save refresh token
            refresh_token: "".to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_username() {
        let offline = Auth::new_offline("offline");
        assert_eq!(offline.get_username(), "offline");

        let mojang = Auth::Mojang {
            auth_type: "mojang".to_owned(),
            username: "mojang".to_string(),
            token: "".to_string(),
            uuid: "".to_string(),
        };
        assert_eq!(mojang.get_username(), "mojang");
    }
}
