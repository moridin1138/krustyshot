//! The custom uploader: send the image to a server the user set up, and get
//! a link back. The settings follow ShareX's custom uploaders closely, so a
//! ShareX `.sxcu` file can be imported as is.
//!
//! The link is found with a template: `$json:data.link$` reads a field of a
//! JSON reply (dots for nesting, `[0]` or `.0` for list items),
//! `$response$` is the whole reply, and anything else is kept as written,
//! e.g. `https://example.com/$json:id$.png`. An empty template means the
//! reply itself is the link. ShareX's older `{json:…}` form works too.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// How the image goes in the request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadBody {
    /// A form with the image in a file field (what most hosts expect).
    #[default]
    Multipart,
    /// The image itself is the request body.
    Binary,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UploadMethod {
    #[default]
    Post,
    Put,
}

/// A custom uploader. Header values often hold API keys; they are stored in
/// the settings file like everything else.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Uploader {
    /// Shown in menus: "Upload to {name}".
    pub name: String,
    pub url: String,
    pub method: UploadMethod,
    pub body: UploadBody,
    /// The form field for the image (multipart only).
    pub file_field: String,
    pub headers: Vec<(String, String)>,
    /// Extra form fields (multipart only).
    pub form: Vec<(String, String)>,
    /// Where the link is in the reply; see the module docs.
    pub link: String,
}

impl Default for Uploader {
    fn default() -> Self {
        Self {
            name: String::new(),
            url: String::new(),
            method: UploadMethod::Post,
            body: UploadBody::Multipart,
            file_field: "file".into(),
            headers: Vec::new(),
            form: Vec::new(),
            link: String::new(),
        }
    }
}

impl Uploader {
    /// Complete enough to offer in menus.
    pub fn is_ready(&self) -> bool {
        !self.name.trim().is_empty() && (self.url.starts_with("https://") || self.url.starts_with("http://"))
    }

    /// Read a ShareX custom uploader (`.sxcu`).
    pub fn from_sxcu(text: &str) -> Result<Self> {
        let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).context("not a ShareX uploader file (JSON)")?;
        let str_of = |key: &str| v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let pairs = |key: &str| -> Vec<(String, String)> {
            v.get(key)
                .and_then(|x| x.as_object())
                .map(|o| o.iter().map(|(k, val)| (k.clone(), val.as_str().map(str::to_string).unwrap_or_else(|| val.to_string()))).collect())
                .unwrap_or_default()
        };
        let url = str_of("RequestURL");
        if url.is_empty() {
            bail!("the file has no RequestURL");
        }
        let body = match str_of("Body").as_str() {
            "" | "MultipartFormData" => UploadBody::Multipart,
            "Binary" => UploadBody::Binary,
            other => bail!("ShareX \"{other}\" uploads aren't supported (only form and binary uploads)"),
        };
        let method = match str_of("RequestMethod").to_uppercase().as_str() {
            "" | "POST" => UploadMethod::Post,
            "PUT" => UploadMethod::Put,
            other => bail!("the {other} method isn't supported (only POST and PUT)"),
        };
        let name = Some(str_of("Name")).filter(|n| !n.is_empty()).unwrap_or_else(|| host_of(&url));
        let file_field = Some(str_of("FileFormName")).filter(|n| !n.is_empty()).unwrap_or_else(|| "file".into());
        Ok(Self { name, url, method, body, file_field, headers: pairs("Headers"), form: pairs("Arguments"), link: str_of("URL") })
    }

    /// Upload `bytes` (a file called `file_name`, of type `mime`) and return
    /// the link. Blocking; call off the UI thread.
    pub fn upload(&self, bytes: &[u8], file_name: &str, mime: &str) -> Result<String> {
        if !self.is_ready() {
            bail!("the uploader needs a name and a web address (https://…)");
        }
        let agent: ureq::Agent =
            ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).http_status_as_error(false).build().into();
        let mut request = match self.method {
            UploadMethod::Post => agent.post(&self.url),
            UploadMethod::Put => agent.put(&self.url),
        }
        .header("User-Agent", concat!("Rustshot/", env!("CARGO_PKG_VERSION")));
        for (k, v) in &self.headers {
            request = request.header(k.as_str(), v.as_str());
        }
        let mut response = match self.body {
            UploadBody::Binary => request.header("Content-Type", mime).send(bytes),
            UploadBody::Multipart => {
                let boundary = format!("rustshot-{:016x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
                let body = multipart(&boundary, &self.form, &self.file_field, file_name, mime, bytes);
                request.header("Content-Type", &format!("multipart/form-data; boundary={boundary}")).send(&body[..])
            }
        }
        .with_context(|| format!("couldn't reach {}", host_of(&self.url)))?;
        let status = response.status();
        let reply = response.body_mut().read_to_string().unwrap_or_default();
        if !status.is_success() {
            let snippet: String = reply.chars().take(300).collect();
            bail!("{} answered {status}: {snippet}", host_of(&self.url));
        }
        let link = extract_link(&self.link, &reply)?;
        if link.is_empty() {
            bail!("the upload worked, but no link was found in the reply");
        }
        Ok(link)
    }
}

/// `example.com` from `https://example.com/upload`.
fn host_of(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url).split(['/', '?', '#']).next().unwrap_or(url).to_string()
}

/// A multipart/form-data body: the extra fields, then the file.
fn multipart(boundary: &str, fields: &[(String, String)], file_field: &str, file_name: &str, mime: &str, bytes: &[u8]) -> Vec<u8> {
    let quote = |s: &str| s.replace('"', "%22").replace(['\r', '\n'], " ");
    let mut body = Vec::with_capacity(bytes.len() + 512);
    for (k, v) in fields {
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{v}\r\n", quote(k)).as_bytes());
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {mime}\r\n\r\n",
            quote(file_field),
            quote(file_name)
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

/// Fill the link template from the server's reply.
fn extract_link(template: &str, reply: &str) -> Result<String> {
    let template = template.trim();
    if template.is_empty() {
        return Ok(reply.trim().to_string());
    }
    let json: Option<serde_json::Value> = serde_json::from_str(reply).ok();
    let mut out = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        // The next `$…$` or `{…}` placeholder.
        let next = [rest.find('$').map(|i| (i, '$', '$')), rest.find('{').map(|i| (i, '{', '}'))].into_iter().flatten().min_by_key(|(i, _, _)| *i);
        let Some((start, _, close)) = next else {
            out.push_str(rest);
            break;
        };
        let Some(len) = rest[start + 1..].find(close) else {
            out.push_str(rest);
            break;
        };
        let inner = &rest[start + 1..start + 1 + len];
        out.push_str(&rest[..start]);
        if inner != "response" && !inner.starts_with("json:") {
            // Not a placeholder: keep the character and look again after it.
            out.push_str(&rest[start..start + 1]);
            rest = &rest[start + 1..];
            continue;
        }
        if inner == "response" {
            out.push_str(reply.trim());
        } else if let Some(path) = inner.strip_prefix("json:") {
            let json = json.as_ref().context("the reply isn't JSON, so the link template can't read it")?;
            let value = json_path(json, path).with_context(|| format!("the reply has no \"{path}\""))?;
            out.push_str(&value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()));
        }
        rest = &rest[start + len + 2..];
    }
    Ok(out.trim().to_string())
}

/// `data.files[0].url` (or `data.files.0.url`) in a JSON value.
fn json_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut v = value;
    for part in path.split('.').filter(|p| !p.is_empty()) {
        let (key, indices) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !key.is_empty() {
            v = match key.parse::<usize>() {
                Ok(n) if v.is_array() => v.get(n)?,
                _ => v.get(key)?,
            };
        }
        for index in indices.split('[').filter(|s| !s.is_empty()) {
            v = v.get(index.trim_end_matches(']').parse::<usize>().ok()?)?;
        }
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_templates() {
        let reply = r#"{"data":{"link":"https://i.example/abc.png","files":[{"id":"x1"}]},"ok":true}"#;
        assert_eq!(extract_link("$json:data.link$", reply).unwrap(), "https://i.example/abc.png");
        assert_eq!(extract_link("{json:data.link}", reply).unwrap(), "https://i.example/abc.png");
        assert_eq!(extract_link("https://h/$json:data.files[0].id$.png", reply).unwrap(), "https://h/x1.png");
        assert_eq!(extract_link("https://h/$json:data.files.0.id$", reply).unwrap(), "https://h/x1");
        assert_eq!(extract_link("", " https://plain.example/1 \n").unwrap(), "https://plain.example/1");
        assert_eq!(extract_link("$response$", "https://r/2").unwrap(), "https://r/2");
        assert!(extract_link("$json:nope$", reply).is_err());
        // Stray dollar signs and braces are left alone.
        assert_eq!(extract_link("$5 {x} $json:data.files[0].id$", reply).unwrap(), "$5 {x} x1");
    }

    #[test]
    fn imports_sharex() {
        let sxcu = r#"{
            "Version": "15.0.0", "Name": "My host", "DestinationType": "ImageUploader",
            "RequestMethod": "POST", "RequestURL": "https://up.example/api/upload",
            "Headers": {"Authorization": "Bearer abc"}, "Body": "MultipartFormData",
            "Arguments": {"expiry": "7d"}, "FileFormName": "image", "URL": "$json:data.url$"
        }"#;
        let u = Uploader::from_sxcu(sxcu).unwrap();
        assert_eq!(u.name, "My host");
        assert_eq!(u.url, "https://up.example/api/upload");
        assert_eq!(u.file_field, "image");
        assert_eq!(u.headers, vec![("Authorization".to_string(), "Bearer abc".to_string())]);
        assert_eq!(u.form, vec![("expiry".to_string(), "7d".to_string())]);
        assert_eq!(u.link, "$json:data.url$");
        assert!(u.is_ready());
        assert!(Uploader::from_sxcu(r#"{"RequestURL":"https://x","Body":"JSON"}"#).is_err());
    }

    /// A real upload to a one-shot server on this machine: checks the
    /// request it gets and reads the link from its JSON reply.
    #[test]
    fn uploads_to_a_local_server() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 4096];
            // Read until the end of the multipart body.
            while !request.windows(4).any(|w| w == b"--\r\n") {
                let n = stream.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
            }
            let reply = r#"{"data":{"link":"https://i.example/up.png"}}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).unwrap();
            String::from_utf8_lossy(&request).into_owned()
        });
        let uploader = Uploader {
            name: "Local".into(),
            url: format!("http://127.0.0.1:{port}/upload"),
            headers: vec![("Authorization".into(), "Bearer t0k".into())],
            form: vec![("album".into(), "x".into())],
            link: "$json:data.link$".into(),
            ..Default::default()
        };
        let link = uploader.upload(b"PNGDATA", "shot.png", "image/png").unwrap();
        assert_eq!(link, "https://i.example/up.png");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /upload HTTP/1.1"), "{request}");
        assert!(request.to_lowercase().contains("authorization: bearer t0k"));
        assert!(request.contains("filename=\"shot.png\""));
        assert!(request.contains("PNGDATA"));
    }

    #[test]
    fn multipart_body_layout() {
        let body = multipart("B", &[("a".into(), "1".into())], "file", "shot.png", "image/png", b"PNG");
        let text = String::from_utf8(body).unwrap();
        assert!(text.starts_with("--B\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n"));
        assert!(text.contains("name=\"file\"; filename=\"shot.png\"\r\nContent-Type: image/png\r\n\r\nPNG\r\n--B--\r\n"));
    }
}
