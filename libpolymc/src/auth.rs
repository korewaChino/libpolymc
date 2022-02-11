pub enum Auth {
    Offline {
        username: String,
    },
    Mojang{
        username: String,
        passowrd: String,
    },
/*     MSFT{
    // note: this is not implemented yet becuase OAuth is a pain
    }, */
}

impl Auth {
    pub fn get_username(&self) -> &String {
      match self {
        Auth::Offline { ref username, ... } => username,
        Auth::Mojang { ref username, ... } => username,
    }
    }
}