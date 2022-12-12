use std::io;

use url::Url;

#[derive(Debug)]
pub struct Uri {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl Uri {
    pub fn from_string(uri: &str) -> io::Result<Self> {
        let url = if uri.find("://").is_none() {
            Url::parse(format!("telnet://{}", uri).as_str())
        } else {
            Url::parse(uri)
        };

        match url {
            Ok(url) => {
                let host = if let Some(host) = url.host_str() {
                    host.to_string()
                } else {
                    return Err(io::ErrorKind::AddrNotAvailable.into());
                };

                let port = if let Some(port) = url.port_or_known_default() {
                    port
                } else {
                    return Err(io::ErrorKind::AddrNotAvailable.into());
                };

                let tls = match url.scheme() {
                    "telnet" => false,
                    "ssl" | "tls" => true,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::AddrNotAvailable,
                            format!("Unexpected scheme: {}", url.scheme()),
                        ))
                    }
                };

                Ok(Self { host, port, tls })
            }

            Err(e) => Err(io::Error::new(io::ErrorKind::AddrNotAvailable, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(raw: &str) -> Uri {
        Uri::from_string(raw).expect(&format!("Failed to parse {}", raw))
    }

    #[test]
    fn simple_test() {
        let uri = parse_ok("thegoodplace.com:12358");
        assert_eq!(uri.host, "thegoodplace.com");
        assert_eq!(uri.port, 12358);
        assert_eq!(uri.tls, false);
    }

    #[test]
    fn ssl_test() {
        let uri = parse_ok("ssl://thegoodplace.com:12358");
        assert_eq!(uri.host, "thegoodplace.com");
        assert_eq!(uri.port, 12358);
        assert_eq!(uri.tls, true);
    }
}
