use std::fmt;

#[derive(Clone, Default)]
pub struct AuthenticateInfo {
    pub(crate) username: String,
    pub(crate) realm: String,
    pub(crate) nonce: String,
    pub(crate) key_hash: String,
}

impl fmt::Debug for AuthenticateInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Authenticate")
            .field("username", &self.username())
            .field("realm", &self.realm())
            .field("nonce", &self.nonce())
            .field("key", &self.key_hash())
            .finish()
    }
}

impl AuthenticateInfo {
    pub fn new<S: Into<String>>(username: S, realm: S, nonce: S, key_hash: S) -> Self {
        Self {
            username: username.into(),
            realm: realm.into(),
            nonce: nonce.into(),
            key_hash: key_hash.into(),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn realm(&self) -> &str {
        &self.realm
    }

    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    pub fn key_hash(&self) -> &str {
        &self.key_hash
    }
}

#[cfg(test)]
mod test {
    use crate::authenticate_info::AuthenticateInfo;

    #[test]
    pub fn test_fmt_debug() {
        let info = AuthenticateInfo::new("zhuwenq", "turn", "11390", "xaaisdgx#a");
        println!("{:?}", info);
    }
}
