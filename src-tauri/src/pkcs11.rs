//! Enumerate client certificates on smart cards / tokens via `pkcs11-tool -O`.
//! Used to populate the "which smart card / key" picker before connecting.

use std::process::Command;

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CertInfo {
  /// PKCS#11 token manufacturer (e.g. `piv_II`).
  pub manufacturer: String,
  /// Object id in hex without `0x` (e.g. `03`), as used in `id=%03`.
  pub id: String,
  pub label: String,
  pub subject: String,
  /// Token label (PIV cardholder, e.g. "Dylan Westra").
  pub token: String,
  /// Friendly device model, e.g. "YubiKey" (from the slot description).
  pub model: String,
  /// PIV slot purpose, e.g. "Key Management".
  pub slot: String,
  /// Certificate expiry as `YYYY-MM-DD` (best-effort).
  pub expiry: String,
}

impl CertInfo {
  /// Common name pulled out of the subject DN.
  pub fn cn(&self) -> String {
    self
      .subject
      .split(',')
      .find_map(|p| p.trim().strip_prefix("CN="))
      .unwrap_or_else(|| self.subject.trim())
      .to_string()
  }

  /// Dropdown label, e.g. "Dylan Westra — YubiKey 9d (slot 03)".
  pub fn display(&self) -> String {
    let cn = self.cn();
    let model = if self.model.is_empty() { "smart card" } else { &self.model };
    format!("{} — {} {} (slot {})", cn, model, piv_slot_code(&self.id), self.id)
  }

  /// Minimal RFC-7512 URI that uniquely selects this cert (no PIN).
  pub fn uri(&self) -> String {
    format!("pkcs11:manufacturer={};id=%{};type=cert", self.manufacturer, self.id)
  }
}

/// PIV slot code for an opensc object id (01→9a, 03→9d, …).
fn piv_slot_code(id: &str) -> String {
  match id {
    "01" => "9a".into(),
    "02" => "9c".into(),
    "03" => "9d".into(),
    "04" => "9e".into(),
    other => other.to_string(),
  }
}

/// PIV slot purpose for an opensc object id.
fn piv_slot(id: &str) -> String {
  match id {
    "01" => "PIV Authentication".into(),
    "02" => "Digital Signature".into(),
    "03" => "Key Management".into(),
    "04" => "Card Authentication".into(),
    other => format!("id {other}"),
  }
}

/// Friendly device name from a pkcs11 slot description line.
fn friendly_model(desc: &str) -> String {
  let d = desc.trim();
  if d.contains("YubiKey") {
    "YubiKey".into()
  } else if d.is_empty() {
    "smart card".into()
  } else {
    d.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
  }
}

/// Common PKCS#11 module locations that exist on this system, for the module
/// picker. Concrete token modules first, p11-kit-proxy last.
pub fn available_modules() -> Vec<String> {
  const CANDIDATES: &[&str] = &[
    // Flatpak-bundled module (the host /usr paths aren't visible in the sandbox).
    "/app/lib/pkcs11/opensc-pkcs11.so",
    "/app/lib/opensc-pkcs11.so",
    // Native host locations.
    "/usr/lib64/pkcs11/opensc-pkcs11.so",
    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so",
    "/usr/lib/pkcs11/opensc-pkcs11.so",
    "/usr/lib64/opensc-pkcs11.so",
    "/usr/lib64/pkcs11/libykcs11.so",
    "/usr/lib/x86_64-linux-gnu/libykcs11.so.2",
    "/usr/lib64/libykcs11.so",
    "/usr/lib64/softhsm/libsofthsm2.so",
    "/usr/lib/softhsm/libsofthsm2.so",
    "/usr/lib/x86_64-linux-gnu/softhsm/libsofthsm2.so",
    "/usr/lib64/p11-kit-proxy.so",
    "/usr/lib/x86_64-linux-gnu/pkcs11/p11-kit-proxy.so",
  ];
  CANDIDATES
    .iter()
    .filter(|p| std::path::Path::new(p).exists())
    .map(|p| p.to_string())
    .collect()
}

/// List certificate objects visible through the given PKCS#11 module, enriched
/// with token label, PIV slot, and (best-effort) subject + expiry.
pub fn enumerate(module: &str) -> Result<Vec<CertInfo>> {
  let out = Command::new("pkcs11-tool")
    .arg("--module")
    .arg(module)
    .arg("-O")
    .output()
    .context("running pkcs11-tool (is opensc installed and the token present?)")?;
  let mut certs = parse(&String::from_utf8_lossy(&out.stdout));

  let (token, model) = token_info(module);
  for c in &mut certs {
    c.token = token.clone();
    c.model = model.clone();
    c.slot = piv_slot(&c.id);
    if let Some((subject, expiry)) = read_cert_meta(module, &c.id) {
      if !subject.is_empty() {
        c.subject = subject;
      }
      c.expiry = expiry;
    }
  }
  Ok(certs)
}

/// The token (cardholder) label and a friendly device model, from `-T`.
fn token_info(module: &str) -> (String, String) {
  let Ok(out) = Command::new("pkcs11-tool").args(["--module", module, "-T"]).output() else {
    return (String::new(), String::new());
  };
  let (mut label, mut model) = (String::new(), String::new());
  for line in String::from_utf8_lossy(&out.stdout).lines() {
    let t = line.trim();
    if let Some(v) = t.strip_prefix("token label") {
      label = v.trim_start_matches([' ', ':']).trim().to_string();
    } else if t.starts_with("Slot ") {
      // "Slot 0 (0x0): Yubico YubiKey OTP+FIDO+CCID 00 00"
      if let Some((_, desc)) = t.split_once("): ") {
        model = friendly_model(desc);
      }
    }
  }
  (label, model)
}

/// Read a cert's DER and parse its subject + expiry via openssl (best-effort).
fn read_cert_meta(module: &str, id: &str) -> Option<(String, String)> {
  use std::io::Write;
  use std::process::Stdio;

  let der = Command::new("pkcs11-tool")
    .args(["--module", module, "--read-object", "--type", "cert", "--id", id])
    .output()
    .ok()?;
  if !der.status.success() || der.stdout.is_empty() {
    return None;
  }

  let mut child = Command::new("openssl")
    .args([
      "x509", "-inform", "DER", "-noout", "-subject", "-enddate", "-nameopt", "RFC2253", "-dateopt",
      "iso_8601",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .ok()?;
  child.stdin.take()?.write_all(&der.stdout).ok()?;
  let out = child.wait_with_output().ok()?;

  let (mut subject, mut expiry) = (String::new(), String::new());
  for line in String::from_utf8_lossy(&out.stdout).lines() {
    if let Some(v) = line.strip_prefix("subject=") {
      subject = v.trim().to_string();
    } else if let Some(v) = line.strip_prefix("notAfter=") {
      // iso_8601: "2027-06-01 12:00:00Z" → keep the date.
      expiry = v.split_whitespace().next().unwrap_or("").to_string();
    }
  }
  Some((subject, expiry))
}

fn parse(text: &str) -> Vec<CertInfo> {
  let mut certs = Vec::new();
  let mut in_cert = false;
  let (mut label, mut subject, mut uri) = (String::new(), String::new(), String::new());

  let flush = |label: &mut String, subject: &mut String, uri: &mut String, certs: &mut Vec<CertInfo>| {
    if let (Some(manufacturer), Some(id)) = (field(uri, "manufacturer="), field(uri, "id=%")) {
      if !id.is_empty() {
        certs.push(CertInfo {
          manufacturer,
          id,
          label: label.clone(),
          subject: subject.clone(),
          ..Default::default()
        });
      }
    }
    label.clear();
    subject.clear();
    uri.clear();
  };

  for line in text.lines() {
    let t = line.trim();
    if t.starts_with("Certificate Object") {
      flush(&mut label, &mut subject, &mut uri, &mut certs);
      in_cert = true;
      continue;
    }
    if t.starts_with("Public Key")
      || t.starts_with("Private Key")
      || t.starts_with("Data object")
      || t.starts_with("Profile object")
    {
      if in_cert {
        flush(&mut label, &mut subject, &mut uri, &mut certs);
      }
      in_cert = false;
      continue;
    }
    if !in_cert {
      continue;
    }
    if let Some(v) = t.strip_prefix("label:") {
      label = v.trim().to_string();
    } else if let Some(v) = t.strip_prefix("subject:") {
      subject = v.trim().trim_start_matches("DN:").trim().to_string();
    } else if let Some(v) = t.strip_prefix("uri:") {
      uri = v.trim().to_string();
    }
  }
  flush(&mut label, &mut subject, &mut uri, &mut certs);
  certs
}

/// Extract `key`'s value from a `;`-delimited PKCS#11 URI.
fn field(uri: &str, key: &str) -> Option<String> {
  let start = uri.find(key)? + key.len();
  let rest = &uri[start..];
  let end = rest.find([';', '?']).unwrap_or(rest.len());
  Some(rest[..end].to_string())
}
