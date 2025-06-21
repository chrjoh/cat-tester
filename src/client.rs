use crate::token;
use crate::token::TokenType;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::Url;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::net::IpAddr;
use std::sync::Arc;
use std::{thread, time::Duration};

pub struct Worker {
    key: String,
    url: String,
    issuer: String,
    ttl: u64,
    token_type: TokenType,
    host: Url,
    cookie_domain: Option<String>,
    max_iterations: u32,
    http_client: reqwest::Client,
    sleep: u64,
}

impl Worker {
    pub fn new(
        key: &str,
        url: &str,
        ttl: u64,
        token_type: TokenType,
        issuer: &str,
        max_iterations: u32,
        sleep: u64,
    ) -> Self {
        let u = url.parse::<Url>().unwrap();
        let scheme = u.scheme();
        let host = u.host_str().unwrap_or("localhost");
        let cookie_host = format!("{}://{}", scheme, host).parse::<Url>().unwrap();
        let cookie_domain = Self::extract_cookie_domain(&host);

        let runner = Self {
            http_client: reqwest::Client::new(), // temporary, will be replaced
            url: String::from(url),
            token_type: token_type.clone(),
            key: String::from(key),
            ttl,
            cookie_domain: cookie_domain.clone(),
            issuer: String::from(issuer),
            host: cookie_host.clone(),
            max_iterations,
            sleep,
        };
        let client = runner
            .create_http_client()
            .expect("Failed to create HTTP client");

        Self {
            key: String::from(key),
            url: String::from(url),
            issuer: String::from(issuer),
            ttl: ttl,
            host: cookie_host,
            token_type: token_type,
            cookie_domain: cookie_domain,
            max_iterations: max_iterations,
            http_client: client,
            sleep,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        if self.token_type == TokenType::Header {
            let token_header = HeaderValue::from_str(&self.encoded_token().unwrap())?;
            headers.insert("CTA-Common-Access-Token", token_header);
        }
        let result = self
            .http_client
            .get(&self.url)
            .headers(headers.clone())
            .send()
            .await?;
        let body = result.text().await?;
        let stream_segment = find_line_after_pattern(&body, "EXTINF").unwrap();
        // Handle that the segments can be a full url or a path segment
        let stream_url = if stream_segment.starts_with("http") {
            stream_segment
        } else {
            replace_last_path_segment(&self.url, &stream_segment)
        };

        for i in 1..self.max_iterations + 1 {
            let res = self
                .http_client
                .get(&stream_url)
                .headers(headers.clone())
                .send()
                .await?;
            if self.token_type == TokenType::Header {
                match res.headers().get("cta-common-access-token") {
                    Some(token) => {
                        headers.try_insert("cta-common-access-token", token.clone())?;
                    }
                    None => {
                        eprintln!("No token found");
                        eprintln!("Headers: {:#?}\n", res.headers());
                    }
                }
            }
            eprintln!(
                "Req: {}, Response: {}, content-length: {:?}",
                i,
                res.status(),
                res.headers().get("content-length").unwrap()
            );
            if self.sleep > 0 {
                thread::sleep(Duration::from_millis(self.sleep));
            }
        }
        Ok(())
    }

    fn encoded_token(&self) -> Option<String> {
        let token_bytes = token::create_token(
            &self.key,
            self.ttl,
            &self.token_type,
            self.cookie_domain.as_ref().unwrap(),
            &self.issuer,
        );
        Some(URL_SAFE_NO_PAD.encode(&token_bytes))
    }

    fn create_http_client(&self) -> reqwest::Result<reqwest::Client> {
        match self.token_type {
            TokenType::Cookie => {
                let token = self.encoded_token().unwrap();
                let cookie_str = format!(
                    "CTA-Common-Access-Token={value}; Domain={domain}; Path=/",
                    value = token,
                    domain = self.cookie_domain.as_ref().unwrap()
                );
                let cookie_store = Arc::new(Jar::default());
                cookie_store.add_cookie_str(&cookie_str, &self.host);
                reqwest::Client::builder()
                    .cookie_provider(cookie_store)
                    .build()
            }
            TokenType::Header => reqwest::Client::builder().build(),
        }
    }

    fn is_ip(s: &str) -> bool {
        s.parse::<IpAddr>().is_ok()
    }

    fn extract_cookie_domain(host: &str) -> Option<String> {
        if Self::is_ip(host) {
            return Some(host.to_string());
        }
        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() >= 2 {
            let domain = format!(".{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
            Some(domain)
        } else {
            None
        }
    }
}

fn replace_last_path_segment(url: &str, path: &str) -> String {
    match url.rfind('/') {
        Some(pos) => format!("{}{}", &url[..=pos], path),
        None => path.to_string(),
    }
}

fn find_line_after_pattern(text: &str, pattern: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    for i in 0..lines.len() - 1 {
        if lines[i].contains(pattern) {
            return Some(lines[i + 1].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn test_run_with_cat_in_header() {
        let server = MockServer::start();

        // Mock the playlist response
        let body = "#EXTM3U\n#EXTINF:10,\nsegment.ts";
        let playlist_mock = server.mock(|when, then| {
            when.method(GET).path("/playlist.m3u8");
            then.status(200)
                .header("content-length", body.len().to_string())
                .body(body);
        });

        // Mock the segment response
        let body = "segment content";
        let segment_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/segment.ts")
                .header_exists("CTA-Common-Access-Token");
            then.status(200)
                .header("content-length", body.len().to_string())
                .body(body);
        });
        let key_hex = "403697de87af64611c1d32a05dab0fe1fcb715a86ab435f1ec99192d79569388"; // "testKey" in hex
        let base_url = server.base_url();
        let runner = Worker::new(
            key_hex,
            &format!("{}/playlist.m3u8", base_url),
            3600,
            TokenType::Header,
            "issuer",
            1,
            0,
        );

        let result = runner.run().await;
        if result.is_err() {
            eprintln!("error {:?}", result);
        }
        assert!(result.is_ok());

        playlist_mock.assert();
        segment_mock.assert();
    }

    #[tokio::test]
    async fn test_run_with_cat_in_cookie() {
        let server = MockServer::start();

        // Mock the playlist response
        let body = "#EXTM3U\n#EXTINF:10,\nsegment.ts";
        let playlist_mock = server.mock(|when, then| {
            when.method(GET).path("/playlist.m3u8");
            then.status(200)
                .header("content-length", body.len().to_string())
                .body(body);
        });

        // Mock the segment response
        let body = "segment content";
        let segment_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/segment.ts")
                .cookie_exists("CTA-Common-Access-Token");
            then.status(200)
                .header("content-length", body.len().to_string())
                .body(body);
        });
        let key_hex = "403697de87af64611c1d32a05dab0fe1fcb715a86ab435f1ec99192d79569388"; // "testKey" in hex
        let base_url = server.base_url();
        let runner = Worker::new(
            key_hex,
            &format!("{}/playlist.m3u8", base_url),
            3600,
            TokenType::Cookie,
            "issuer",
            1,
            0,
        );
        let result = runner.run().await;
        if result.is_err() {
            eprintln!("error {:?}", result);
        }
        assert!(result.is_ok());

        playlist_mock.assert();
        segment_mock.assert();
    }

    #[test]
    fn get_cookie_domain_from_host() {
        let host = "www.host1.example.com";
        let res = Worker::extract_cookie_domain(host);
        assert_eq!(res, Some(".example.com".to_string()));
    }

    #[test]
    fn get_cookie_domain_from_host_if_ip() {
        let host = "127.0.0.1";
        let res = Worker::extract_cookie_domain(host);
        assert_eq!(res, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn replace_last_segment() {
        let url = "https://my.test.domain.com/first/second/last.ism";
        let last_path = "replaced.ism";
        let result = replace_last_path_segment(&url, &last_path);
        assert_eq!(
            result,
            "https://my.test.domain.com/first/second/replaced.ism"
        );
    }

    #[test]
    fn parse_m3u8_response() {
        let body_two = r#"
        #EXTM3U
        #EXTM3U
        #EXT-X-VERSION:6
        ## Created with Unified Streaming Platform  (version=1.13.0-29687)
        #EXT-X-MEDIA-SEQUENCE:455767831
        #EXT-X-INDEPENDENT-SEGMENTS
        #EXT-X-TARGETDURATION:6
        #USP-X-TIMESTAMP-MAP:MPEGTS=8322344688,LOCAL=2025-06-17T08:21:08.040000Z
        #EXT-X-MAP:URI="hls/20465721-video=5000000.m4s"
        #EXT-X-PROGRAM-DATE-TIME:2025-06-17T08:21:08.040000Z
        #EXT-X-KEY:METHOD=SAMPLE-AES,KEYID=0x706fe4f5a1fc3ad2af49a6698b822bad,URI="data:text/plain;base64,AAAAdXBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAFUSEHBv5PWh/DrSr0mmaYuCK60aCGNhc3RsYWJzIihleUpoYzNObGRFbGtJam9pZEhadFpXUnBZUzB5TURRMk5UY3lNU0o5MgdkZWZhdWx0SPPGiZsG",KEYFORMAT="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed",KEYFORMATVERSIONS="1"
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI="data:text/plain;charset=UTF-16;base64,KgMAAAEAAQAgAzwAVwBSAE0ASABFAEEARABFAFIAIAB2AGUAcgBzAGkAbwBuAD0AIgA0AC4AMwAuADAALgAwACIAIAB4AG0AbABuAHMAPQAiAGgAdAB0AHAAOgAvAC8AcwBjAGgAZQBtAGEAcwAuAG0AaQBjAHIAbwBzAG8AZgB0AC4AYwBvAG0ALwBEAFIATQAvADIAMAAwADcALwAwADMALwBQAGwAYQB5AFIAZQBhAGQAeQBIAGUAYQBkAGUAcgAiAD4APABEAEEAVABBAD4APABQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsASQBEAFMAPgA8AEsASQBEACAAVgBBAEwAVQBFAD0AIgA5AGUAUgB2AGMAUAB5AGgAMABqAHEAdgBTAGEAWgBwAGkANABJAHIAcgBRAD0APQAiACAAQQBMAEcASQBEAD0AIgBBAEUAUwBDAEIAQwAiACAALwA+ADwALwBLAEkARABTAD4APAAvAFAAUgBPAFQARQBDAFQASQBOAEYATwA+ADwATABBAF8AVQBSAEwAPgBoAHQAdABwAHMAOgAvAC8AbABpAGMALgBkAHIAbQB0AG8AZABhAHkALgBjAG8AbQAvAGwAaQBjAGUAbgBzAGUALQBwAHIAbwB4AHkALQBoAGUAYQBkAGUAcgBhAHUAdABoAC8AZAByAG0AdABvAGQAYQB5AC8AUgBpAGcAaAB0AHMATQBhAG4AYQBnAGUAcgAuAGEAcwBtAHgAPAAvAEwAQQBfAFUAUgBMAD4APABMAFUASQBfAFUAUgBMAD4AaAB0AHQAcABzADoALwAvAHAAbABhAHkAcgBlAGEAZAB5AC0AdQBpAC4AZQB4AGEAbQBwAGwAZQAuAGMAbwBtADwALwBMAFUASQBfAFUAUgBMAD4APABEAEUAQwBSAFkAUABUAE8AUgBTAEUAVABVAFAAPgBPAE4ARABFAE0AQQBOAEQAPAAvAEQARQBDAFIAWQBQAFQATwBSAFMARQBUAFUAUAA+ADwALwBEAEEAVABBAD4APAAvAFcAUgBNAEgARQBBAEQARQBSAD4A",KEYFORMAT="com.microsoft.playready",KEYFORMATVERSIONS="1"
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI="skd://drmtoday?assetId=tvmedia-20465721&variantId&keyId=706fe4f5a1fc3ad2af49a6698b822bad",KEYFORMAT="com.apple.streamingkeydelivery",KEYFORMATVERSIONS="1"
        #EXT-X-KEY:METHOD=SAMPLE-AES,KEYID=0x706fe4f5a1fc3ad2af49a6698b822bad,URI="data:text/plain;base64,AAAAn3Bzc2gAAAAAPV5tNZuaQei4Q908bnLELAAAAH97InZlcnNpb24iOiJWMS4wIiwia2lkcyI6WyJjRy9rOWFIOE90S3ZTYVpwaTRJcnJRPT0iXSwiY29udGVudElEIjoiZXlKaGMzTmxkRWxrSWpvaWRIWnRaV1JwWVMweU1EUTJOVGN5TVNKOSIsImVuc2NoZW1hIjoiY2JjcyJ9",IV=0xE105A618D09DC0CCFFCDBCCA711E6BD0,KEYFORMAT="urn:uuid:3d5e6d35-9b9a-41e8-b843-dd3c6e72c42c",KEYFORMATVERSIONS="1"
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767831.m4s
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767832.m4s
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767833.m4s
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767834.m4s#EXT-X-VERSION:6
        ## Created with Unified Streaming Platform  (version=1.13.0-29687)
        #EXT-X-MEDIA-SEQUENCE:455767831
        #EXT-X-INDEPENDENT-SEGMENTS
        #EXT-X-TARGETDURATION:6
        #USP-X-TIMESTAMP-MAP:MPEGTS=8322344688,LOCAL=2025-06-17T08:21:08.040000Z
        #EXT-X-MAP:URI="hls/20465721-video=5000000.m4s"
        #EXT-X-PROGRAM-DATE-TIME:2025-06-17T08:21:08.040000Z
        #EXT-X-KEY:METHOD=SAMPLE-AES,KEYID=0x706fe4f5a1fc3ad2af49a6698b822bad,URI="data:text/plain;base64,AAAAdXBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAFUSEHBv5PWh/DrSr0mmaYuCK60aCGNhc3RsYWJzIihleUpoYzNObGRFbGtJam9pZEhadFpXUnBZUzB5TURRMk5UY3lNU0o5MgdkZWZhdWx0SPPGiZsG",KEYFORMAT="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed",KEYFORMATVERSIONS="1"
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI="data:text/plain;charset=UTF-16;base64,KgMAAAEAAQAgAzwAVwBSAE0ASABFAEEARABFAFIAIAB2AGUAcgBzAGkAbwBuAD0AIgA0AC4AMwAuADAALgAwACIAIAB4AG0AbABuAHMAPQAiAGgAdAB0AHAAOgAvAC8AcwBjAGgAZQBtAGEAcwAuAG0AaQBjAHIAbwBzAG8AZgB0AC4AYwBvAG0ALwBEAFIATQAvADIAMAAwADcALwAwADMALwBQAGwAYQB5AFIAZQBhAGQAeQBIAGUAYQBkAGUAcgAiAD4APABEAEEAVABBAD4APABQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsASQBEAFMAPgA8AEsASQBEACAAVgBBAEwAVQBFAD0AIgA5AGUAUgB2AGMAUAB5AGgAMABqAHEAdgBTAGEAWgBwAGkANABJAHIAcgBRAD0APQAiACAAQQBMAEcASQBEAD0AIgBBAEUAUwBDAEIAQwAiACAALwA+ADwALwBLAEkARABTAD4APAAvAFAAUgBPAFQARQBDAFQASQBOAEYATwA+ADwATABBAF8AVQBSAEwAPgBoAHQAdABwAHMAOgAvAC8AbABpAGMALgBkAHIAbQB0AG8AZABhAHkALgBjAG8AbQAvAGwAaQBjAGUAbgBzAGUALQBwAHIAbwB4AHkALQBoAGUAYQBkAGUAcgBhAHUAdABoAC8AZAByAG0AdABvAGQAYQB5AC8AUgBpAGcAaAB0AHMATQBhAG4AYQBnAGUAcgAuAGEAcwBtAHgAPAAvAEwAQQBfAFUAUgBMAD4APABMAFUASQBfAFUAUgBMAD4AaAB0AHQAcABzADoALwAvAHAAbABhAHkAcgBlAGEAZAB5AC0AdQBpAC4AZQB4AGEAbQBwAGwAZQAuAGMAbwBtADwALwBMAFUASQBfAFUAUgBMAD4APABEAEUAQwBSAFkAUABUAE8AUgBTAEUAVABVAFAAPgBPAE4ARABFAE0AQQBOAEQAPAAvAEQARQBDAFIAWQBQAFQATwBSAFMARQBUAFUAUAA+ADwALwBEAEEAVABBAD4APAAvAFcAUgBNAEgARQBBAEQARQBSAD4A",KEYFORMAT="com.microsoft.playready",KEYFORMATVERSIONS="1"
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI="skd://drmtoday?assetId=tvmedia-20465721&variantId&keyId=706fe4f5a1fc3ad2af49a6698b822bad",KEYFORMAT="com.apple.streamingkeydelivery",KEYFORMATVERSIONS="1"
        #EXT-X-KEY:METHOD=SAMPLE-AES,KEYID=0x706fe4f5a1fc3ad2af49a6698b822bad,URI="data:text/plain;base64,AAAAn3Bzc2gAAAAAPV5tNZuaQei4Q908bnLELAAAAH97InZlcnNpb24iOiJWMS4wIiwia2lkcyI6WyJjRy9rOWFIOE90S3ZTYVpwaTRJcnJRPT0iXSwiY29udGVudElEIjoiZXlKaGMzTmxkRWxrSWpvaWRIWnRaV1JwWVMweU1EUTJOVGN5TVNKOSIsImVuc2NoZW1hIjoiY2JjcyJ9",IV=0xE105A618D09DC0CCFFCDBCCA711E6BD0,KEYFORMAT="urn:uuid:3d5e6d35-9b9a-41e8-b843-dd3c6e72c42c",KEYFORMATVERSIONS="1"
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767831.m4s
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767832.m4s
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767833.m4s
        #EXTINF:3.84, no desc
        hls/20465721-video=5000000-455767834.m4s
        "#;
        let res = find_line_after_pattern(body_two, "EXTINF");
        assert_eq!(res.unwrap(), "hls/20465721-video=5000000-455767831.m4s");
    }
}
