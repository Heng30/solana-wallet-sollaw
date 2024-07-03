use reqwest::{
    header::{HeaderMap, ACCEPT, CACHE_CONTROL, USER_AGENT},
    Client, Proxy, Result,
};

pub enum ProxyType {
    Http,
    Socks5,
    Unknown,
}

impl From<&str> for ProxyType {
    fn from(pt: &str) -> Self {
        match pt.to_lowercase().as_str() {
            "http" => ProxyType::Http,
            "socks5" => ProxyType::Socks5,
            _ => ProxyType::Unknown,
        }
    }
}

#[allow(dead_code)]
pub fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert(ACCEPT, "*/*".parse().unwrap());

    headers.insert(CACHE_CONTROL, "no-cache".parse().unwrap());
    headers
}

#[allow(dead_code)]
pub fn client(conf: Option<(ProxyType, String, u16)>) -> Result<Client> {
    match conf {
        Some((proxy, url, port)) => {
            let proxy_url = match proxy {
                ProxyType::Http => Proxy::all(format!("http://{}:{}", url, port))?,
                ProxyType::Socks5 => Proxy::all(format!("socks5://{}:{}", url, port))?,
                _ => return Ok(Client::new()),
            };
            Ok(Client::builder().proxy(proxy_url).build()?)
        }
        None => Ok(Client::new()),
    }
}
