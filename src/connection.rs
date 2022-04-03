use std::io;

pub struct Uri {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl Uri {
    pub fn from_string(uri: &str) -> io::Result<Self> {
        Ok(Self {
            host: uri.to_string(),
            port: 5656,
            tls: false,
        })
    }
}
