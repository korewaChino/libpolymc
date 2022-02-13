// use HTTP for logging in?
use serde_json::{json, Value};


pub enum LoginRequest{
    Mojang {
        username: String,
        password: String,
    },
    Msft {
        client_id: String,
        redirect_uri: String,
        state: String,
    },
    MsftToken {
        client_id: String,
        client_secret: String,
        code: String,
        redirect_uri: String,
    },
    MsftRefresh {
        client_id: String,
        client_secret: String,
        refresh_token: String,
        redirect_uri: String,
    }
}

impl LoginRequest {
    // login requests come in 2 types: mojang and msft
    // msft uses HTTP options to input data because OAuth
    // mojang sends a POST request with JSON data

    pub fn new_login(&self) -> String{
        match self {
            LoginRequest::Mojang { username, password } => {
                let data = json!({
                    "agent": {
                        "name": "Minecraft",
                        "version": 1
                    },
                    "username": username,
                    "password": password,
                });
                data.to_string()
            },
            LoginRequest::Msft { client_id, redirect_uri, state } => {
                let mut opts = Vec::<String>::new();
                opts.push(format!("client_id={}", client_id));
                opts.push("response_type=code".to_string());
                opts.push(format!("redirect_uri={}", redirect_uri));
                opts.push("scope=XboxLive.signin%20offline_access".to_string());
                opts.push(format!("state={}", state));
                opts.join("&")
            }
            LoginRequest::MsftToken { client_id, client_secret, code, redirect_uri } => {
                let mut opts = Vec::<String>::new();
                opts.push(format!("client_id={}", client_id));
                opts.push(format!("client_secret={}", client_secret));
                opts.push(format!("code={}", code));
                opts.push(format!("redirect_uri={}", redirect_uri));
                opts.push("grant_type=authorization_code".to_string());
                opts.join("&")
            },
            LoginRequest::MsftRefresh { client_id, client_secret, refresh_token, redirect_uri } => {
                let mut opts = Vec::<String>::new();
                opts.push(format!("client_id={}", client_id));
                opts.push(format!("client_secret={}", client_secret));
                opts.push(format!("refresh_token={}", refresh_token));
                opts.push("grant_type=refresh_token".to_string());
                opts.push(format!("redirect_uri={}", redirect_uri));
                opts.join("&")
            },
        }
    }
}


pub enum Auth {
    Offline { username: String },
    Mojang { username: String, token: String },
    MSFT { token: String },
}

impl Auth {
    /// Create a new offline user.
    pub fn new_offline(name: &str) -> Self {
        Auth::Offline {
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_username() {
        let offline = Auth::new_offline("offline");
        assert_eq!(offline.get_username(), "offline");

        let mojang = Auth::Mojang {
            username: "mojang".to_string(),
            token: "".to_string(),
        };
        assert_eq!(mojang.get_username(), "mojang");
    }
}
