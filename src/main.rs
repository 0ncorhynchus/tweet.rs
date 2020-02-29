use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::header;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{SystemTime, SystemTimeError};
use url::Url;

const UPDATE_URL: &'static str = "https://api.twitter.com/1.1/statuses/update.json";

fn get_timestamp() -> Result<String, SystemTimeError> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    Ok(timestamp.to_string())
}

fn gen_nonce() -> String {
    const NONCE_LEN: usize = 20;
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(NONCE_LEN)
        .collect()
}

fn percent_encode(input: &str) -> String {
    const FRAGMENTS: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');
    utf8_percent_encode(input, FRAGMENTS).to_string()
}

fn gen_signature(key: String, url: &str, params: &str) -> String {
    let signature_data = format!(
        "{}&{}&{}",
        percent_encode("POST"),
        percent_encode(url),
        percent_encode(params)
    );
    percent_encode(&base64::encode(&hmacsha1::hmac_sha1(
        key.as_bytes(),
        signature_data.as_bytes(),
    )))
}

#[derive(Debug, Deserialize)]
struct Config {
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    access_token_secret: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = envy::from_env::<Config>()?;
    let status = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let timestamp = percent_encode(&get_timestamp()?);
    let nonce = percent_encode(&gen_nonce());

    let mut params: HashMap<String, String> = vec![
        (
            "oauth_consumer_key".to_string(),
            config.consumer_key.clone(),
        ),
        ("oauth_token".to_string(), config.access_token.clone()),
        (
            "oauth_signature_method".to_string(),
            "HMAC-SHA1".to_string(),
        ),
        ("oauth_timestamp".to_string(), timestamp),
        ("oauth_nonce".to_string(), nonce),
        ("oauth_version".to_string(), "1.0".to_string()),
    ]
    .into_iter()
    .collect();

    let mut params_str = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>();
    params_str.push(format!("status={}", percent_encode(&status)));
    params_str.sort();
    let params_str = &params_str.join("&");

    let signature_key = format!(
        "{}&{}",
        percent_encode(&config.consumer_secret),
        percent_encode(&config.access_token_secret)
    );
    let signature = gen_signature(signature_key, UPDATE_URL, &params_str);

    params.insert("oauth_signature".to_string(), signature);

    let mut header_value = params
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, v))
        .collect::<Vec<_>>();
    header_value.sort();
    let header_value = "OAuth ".to_string() + &header_value.join(", ");

    let mut headers = header::HeaderMap::new();
    headers.insert(header::AUTHORIZATION, header_value.parse()?);

    let url = Url::parse_with_params(UPDATE_URL, &[("status", status)])?;

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()?;
    let res = client.post(url).send()?;
    println!("response: {:?}", res);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_encode() {
        assert_eq!(
            percent_encode("Ladies + Gentlemen"),
            "Ladies%20%2B%20Gentlemen"
        );
        assert_eq!(
            percent_encode("An encoded string!"),
            "An%20encoded%20string%21"
        );
        assert_eq!(
            percent_encode("Dogs, Cats & Mice"),
            "Dogs%2C%20Cats%20%26%20Mice"
        );
        assert_eq!(percent_encode("☃"), "%E2%98%83");
    }
}
