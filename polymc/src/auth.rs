pub enum Auth {
    Offline { username: String },
    Mojang { username: String, token: String },
    //MSFT{
    // note: this is not implemented yet because OAuth is a pain
    //},
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
        }
    }

    pub fn get_token(&self) -> Option<&str> {
        match self {
            Auth::Offline { .. } => None,
            Auth::Mojang { token, .. } => Some(token),
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
