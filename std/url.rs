impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[allow(dead_code)]
fn oneway_url_parse(s: String) -> Result<Url, url::ParseError> {
    url::Url::parse(&s).map(|_| Url(s))
}
