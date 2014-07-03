extern crate url;
extern crate curl;

use url::Url;
use std::collections::HashMap;
use std::io::MemReader;

use curl::http;

/// Configuration of an oauth2 application.
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub auth_url: Url,
    pub token_url: Url,
    pub redirect_url: String,
}

#[deriving(Show, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct Token {
    pub access_token: String,
    pub scopes: Vec<String>,
    pub token_type: String,
}

/// Helper trait for extending the builder-style pattern of curl::Request.
///
/// This trait allows chaining the correct authorization headers onto a curl
/// request via the builder style.
pub trait Authorization {
    fn auth_with(self, token: &Token) -> Self;
}

impl Config {
    pub fn new(id: &str, secret: &str, auth_url: &str,
               token_url: &str) -> Config {
        Config {
            client_id: id.to_string(),
            client_secret: secret.to_string(),
            scopes: Vec::new(),
            auth_url: from_str(auth_url).unwrap(),
            token_url: from_str(token_url).unwrap(),
            redirect_url: String::new(),
        }
    }

    pub fn authorize_url(&self, state: String) -> Url {
        let mut url = self.auth_url.clone();
        url.query.push(("client_id".to_string(), self.client_id.clone()));
        url.query.push(("state".to_string(), state));
        url.query.push(("scope".to_string(), self.scopes.connect(",")));
        if self.redirect_url.len() > 0 {
            url.query.push(("redirect_uri".to_string(),
                            self.redirect_url.clone()));
        }
        return url;
    }

    pub fn exchange(&self, code: String) -> Result<Token, String> {
        let mut form = HashMap::new();
        form.insert("client_id".to_string(), vec![self.client_id.clone()]);
        form.insert("client_secret".to_string(), vec![self.client_secret.clone()]);
        form.insert("code".to_string(), vec![code]);
        if self.redirect_url.len() > 0 {
            form.insert("redirect_uri".to_string(),
                        vec![self.redirect_url.clone()]);
        }

        let form = url::encode_form_urlencoded(&form);
        let mut form = MemReader::new(form.into_bytes());

        let result = try!(http::handle()
                               .post(self.token_url.to_str().as_slice(),
                                     &mut form as &mut Reader)
                               .header("Content-Type",
                                       "application/x-www-form-urlencoded")
                               .exec()
                               .map_err(|s| s.to_str()));

        if result.get_code() != 200 {
            return Err(format!("expected `200`, found `{}`", result.get_code()))
        }

        let mut token = Token {
            access_token: String::new(),
            scopes: Vec::new(),
            token_type: String::new(),
        };

        for(k, v) in url::decode_form_urlencoded(result.get_body()).move_iter() {
            let v = match v.move_iter().next() { Some(v) => v, None => continue };
            match k.as_slice() {
                "access_token" => token.access_token = v,
                "token_type" => token.token_type = v,
                "scope" => {
                    token.scopes = v.as_slice().split(',')
                                    .map(|s| s.to_string()).collect();
                }
                _ => {}
            }
        }

        if token.access_token.len() == 0 {
            Err(format!("could not find access_token in the response"))
        } else {
            Ok(token)
        }

    }
}

impl<'a, 'b> Authorization for http::Request<'a, 'b> {
    fn auth_with(self, token: &Token) -> http::Request<'a, 'b> {
        self.header("Authorization",
                    format!("token {}", token.access_token).as_slice())
    }
}
