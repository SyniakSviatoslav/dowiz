//! P67 Wave-0 adapters: Hetzner Cloud and Cloudflare Tunnel, for real.
//!
//! WHY THEY ARE NOT IN `dowiz-core`. The ports (`VpsProvider`, `TunnelProvider`)
//! live there, and their Wave-0 implementations there return
//! `Err(Unauthorized)` -- which reads like laziness and is not. `dowiz-core` is
//! the pure side: MANIFESTO C2 keeps clock, RNG, float and NETWORK vocabulary
//! out of it, so a crate that decides money and order state cannot also open a
//! socket. An adapter that talks to an HTTP API therefore cannot live there. It
//! lives here, where the server already dials out, and the pure crate keeps the
//! shape.
//!
//! The ports are also SYNC, because a pure fold has nothing to await. These are
//! async, because a provisioning call takes seconds. Rather than bridge one
//! onto the other with a blocking pool -- which would put a runtime inside the
//! pure crate's contract -- the adapters mirror the port's METHODS and its error
//! type, and the orchestration that used to be imagined as a trait object is an
//! ordinary async function.
//!
//! CREDENTIALS COME FROM THE ENVIRONMENT AND NOWHERE ELSE. Nothing here has a
//! default account, a default zone or a baked-in token. An operator without
//! credentials gets a named refusal at the first call rather than a request to
//! somebody else's account.

use serde_json::{json, Value};

use crate::httpc::{self, HttpError};

#[derive(Debug)]
pub enum ProvisionError {
    /// No credential configured for this provider.
    NotConfigured(&'static str),
    /// The provider answered and refused. Carries its own message, because
    /// "token lacks permission" and "server type unavailable in this location"
    /// need different people to do different things.
    Rejected { status: u16, message: String },
    Transport(String),
    /// The provider answered 2xx with something this cannot read.
    Unexpected(String),
}

impl std::fmt::Display for ProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvisionError::NotConfigured(v) => {
                write!(f, "{v} is not set; provisioning needs a real credential")
            }
            ProvisionError::Rejected { status, message } => write!(f, "provider said {status}: {message}"),
            ProvisionError::Transport(e) => write!(f, "could not reach the provider: {e}"),
            ProvisionError::Unexpected(e) => write!(f, "unreadable answer: {e}"),
        }
    }
}

fn transport(e: HttpError) -> ProvisionError {
    ProvisionError::Transport(e.to_string())
}

/// Read a provider's error text out of whatever shape it uses.
///
/// Hetzner answers `{"error":{"message":...}}`, Cloudflare
/// `{"errors":[{"message":...}]}`. Both are handled because guessing wrong here
/// turns a precise, actionable message into "something went wrong".
fn provider_message(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return text.chars().take(300).collect();
    };
    if let Some(m) = v.get("error").and_then(|e| e.get("message")).and_then(Value::as_str) {
        return m.to_string();
    }
    if let Some(list) = v.get("errors").and_then(Value::as_array) {
        let joined: Vec<String> = list
            .iter()
            .filter_map(|e| e.get("message").and_then(Value::as_str).map(str::to_string))
            .collect();
        if !joined.is_empty() {
            return joined.join("; ");
        }
    }
    text.chars().take(300).collect()
}

async fn call(
    method: &str,
    url: &str,
    token: &str,
    body: Option<Value>,
) -> Result<Value, ProvisionError> {
    let auth = format!("Bearer {token}");
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let (code, raw) = httpc::request(
        method,
        url,
        &[("authorization", &auth), ("content-type", "application/json")],
        payload.as_bytes(),
        // Provisioning is slow and the alternative to waiting is a half-created
        // server nobody knows about.
        60_000,
        1024 * 1024,
    )
    .await
    .map_err(transport)?;

    if !(200..300).contains(&code) {
        return Err(ProvisionError::Rejected { status: code, message: provider_message(&raw) });
    }
    if raw.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&raw)
        .map_err(|e| ProvisionError::Unexpected(format!("{e}: {}", String::from_utf8_lossy(&raw).chars().take(200).collect::<String>())))
}

// ── Hetzner Cloud ────────────────────────────────────────────────────────────

/// The VPS a hub runs on.
pub struct Hetzner {
    token: String,
}

/// `Debug` is written rather than derived, and it prints `<set>` for the token.
///
/// A derived one would put a live cloud credential into any log line, panic
/// message or test failure that happened to format this struct -- and a Hetzner
/// token can create and destroy machines. Same rule as the AI assistant's.
impl std::fmt::Debug for Hetzner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hetzner").field("token", &"<set>").finish()
    }
}

/// What to create. Defaults chosen for ONE RESTAURANT, not for a platform: the
/// smallest shared-vCPU Arm instance Hetzner sells is ample for a hub whose
/// whole dataset is four files and whose traffic is one venue's customers.
#[derive(Debug, Clone)]
pub struct ServerSpec {
    pub name: String,
    pub server_type: String,
    pub location: String,
    pub image: String,
    /// cloud-init, run on first boot. This is how the hub binary and its
    /// systemd unit get onto a fresh machine without an SSH session.
    pub user_data: Option<String>,
    pub ssh_key_ids: Vec<i64>,
}

impl Default for ServerSpec {
    fn default() -> Self {
        ServerSpec {
            name: "dowiz-hub".into(),
            // cax11: 2 Arm vCPU, 4 GB, ~4 EUR/month at time of writing. Arm
            // because the hub is Rust and cross-compiles cleanly, and because
            // it is roughly half the price of the x86 equivalent.
            server_type: "cax11".into(),
            // Falkenstein: the closest Hetzner region to Albania.
            location: "fsn1".into(),
            image: "debian-12".into(),
            user_data: None,
            ssh_key_ids: Vec::new(),
        }
    }
}

impl Hetzner {
    /// `HETZNER_API_TOKEN`, or a named refusal.
    pub fn from_env() -> Result<Hetzner, ProvisionError> {
        let token = std::env::var("HETZNER_API_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty())
            .ok_or(ProvisionError::NotConfigured("HETZNER_API_TOKEN"))?;
        Ok(Hetzner { token })
    }

    const BASE: &'static str = "https://api.hetzner.cloud/v1";

    /// Create the server a hub will live on.
    pub async fn create_server(&self, spec: &ServerSpec) -> Result<i64, ProvisionError> {
        let mut body = json!({
            "name": spec.name,
            "server_type": spec.server_type,
            "location": spec.location,
            "image": spec.image,
            // Public IPv4 costs money and a hub reached through a Cloudflare
            // tunnel does not need one. IPv6 stays for outbound.
            "public_net": { "enable_ipv4": false, "enable_ipv6": true },
            "start_after_create": true,
            "labels": { "managed-by": "dowiz", "role": "hub" }
        });
        if let Some(ud) = &spec.user_data {
            body["user_data"] = json!(ud);
        }
        if !spec.ssh_key_ids.is_empty() {
            body["ssh_keys"] = json!(spec.ssh_key_ids);
        }
        let v = call("POST", &format!("{}/servers", Self::BASE), &self.token, Some(body)).await?;
        v.get("server")
            .and_then(|s| s.get("id"))
            .and_then(Value::as_i64)
            .ok_or_else(|| ProvisionError::Unexpected(format!("no server id in {v}")))
    }

    pub async fn delete_server(&self, id: i64) -> Result<(), ProvisionError> {
        call("DELETE", &format!("{}/servers/{id}", Self::BASE), &self.token, None).await?;
        Ok(())
    }

    /// Snapshot, then release the machine -- the "suspended but preserved"
    /// state the P67 port calls `suspend_preserving`.
    ///
    /// The ORDER is the whole point: a snapshot that fails must leave the server
    /// running. Deleting first and snapshotting after would destroy a venue's
    /// data to save four euros a month.
    pub async fn snapshot(&self, id: i64, description: &str) -> Result<i64, ProvisionError> {
        let v = call(
            "POST",
            &format!("{}/servers/{id}/actions/create_image", Self::BASE),
            &self.token,
            Some(json!({ "type": "snapshot", "description": description,
                         "labels": { "managed-by": "dowiz" } })),
        )
        .await?;
        v.get("image")
            .and_then(|i| i.get("id"))
            .and_then(Value::as_i64)
            .ok_or_else(|| ProvisionError::Unexpected(format!("no image id in {v}")))
    }

    /// Every server this account has that dowiz created. Filtered by LABEL, so
    /// an operator's other machines are never listed and never touchable.
    pub async fn list_hubs(&self) -> Result<Vec<(i64, String, String)>, ProvisionError> {
        let v = call(
            "GET",
            &format!("{}/servers?label_selector=managed-by%3Ddowiz", Self::BASE),
            &self.token,
            None,
        )
        .await?;
        Ok(v.get("servers")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|s| {
                        Some((
                            s.get("id")?.as_i64()?,
                            s.get("name")?.as_str()?.to_string(),
                            s.get("status")?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}

// ── Cloudflare Tunnel ────────────────────────────────────────────────────────

/// The way a hub on a machine with no public IPv4 is reachable from a phone.
pub struct CloudflareTunnel {
    account_id: String,
    zone_id: String,
    token: String,
}

/// Account and zone ids are printed -- they identify which account, and an
/// operator debugging a wrong-account error needs to see them. The TOKEN is not.
impl std::fmt::Debug for CloudflareTunnel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudflareTunnel")
            .field("account_id", &self.account_id)
            .field("zone_id", &self.zone_id)
            .field("token", &"<set>")
            .finish()
    }
}

impl CloudflareTunnel {
    pub fn from_env() -> Result<CloudflareTunnel, ProvisionError> {
        let token = std::env::var("CLOUDFLARE_API_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty())
            .ok_or(ProvisionError::NotConfigured("CLOUDFLARE_API_TOKEN"))?;
        let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID")
            .ok()
            .filter(|t| !t.trim().is_empty())
            .ok_or(ProvisionError::NotConfigured("CLOUDFLARE_ACCOUNT_ID"))?;
        // The zone is only needed to point a hostname at the tunnel; a tunnel
        // can be created and configured without one.
        let zone_id = std::env::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();
        Ok(CloudflareTunnel { account_id, zone_id, token })
    }

    const BASE: &'static str = "https://api.cloudflare.com/client/v4";

    /// Create a tunnel and return its id and its connector token.
    ///
    /// `config_src: "cloudflare"` means the ingress rules are held by
    /// Cloudflare rather than in a file on the VPS -- which is what lets a
    /// venue's routing be changed without touching their machine.
    pub async fn create_tunnel(&self, name: &str) -> Result<(String, String), ProvisionError> {
        let v = call(
            "POST",
            &format!("{}/accounts/{}/cfd_tunnel", Self::BASE, self.account_id),
            &self.token,
            Some(json!({ "name": name, "config_src": "cloudflare" })),
        )
        .await?;
        let r = v.get("result").ok_or_else(|| ProvisionError::Unexpected(format!("{v}")))?;
        let id = r.get("id").and_then(Value::as_str)
            .ok_or_else(|| ProvisionError::Unexpected("no tunnel id".into()))?;
        let token = r.get("token").and_then(Value::as_str)
            .ok_or_else(|| ProvisionError::Unexpected("no connector token".into()))?;
        Ok((id.to_string(), token.to_string()))
    }

    /// Point the tunnel at the hub's local port.
    ///
    /// The trailing catch-all `http_status:404` is REQUIRED by Cloudflare: a
    /// rule set without one is rejected, and it is also what stops an
    /// unmatched hostname reaching the hub at all.
    pub async fn configure_ingress(
        &self,
        tunnel_id: &str,
        hostname: &str,
        local: &str,
    ) -> Result<(), ProvisionError> {
        call(
            "PUT",
            &format!("{}/accounts/{}/cfd_tunnel/{tunnel_id}/configurations", Self::BASE, self.account_id),
            &self.token,
            Some(json!({
                "config": {
                    "ingress": [
                        { "hostname": hostname, "service": local },
                        { "service": "http_status:404" }
                    ]
                }
            })),
        )
        .await?;
        Ok(())
    }

    /// A CNAME from the venue's hostname to the tunnel.
    ///
    /// `proxied` is true and must be: an unproxied record would publish the
    /// tunnel's address and defeat the point of not having a public IP.
    pub async fn route_dns(&self, hostname: &str, tunnel_id: &str) -> Result<String, ProvisionError> {
        if self.zone_id.is_empty() {
            return Err(ProvisionError::NotConfigured("CLOUDFLARE_ZONE_ID"));
        }
        let v = call(
            "POST",
            &format!("{}/zones/{}/dns_records", Self::BASE, self.zone_id),
            &self.token,
            Some(json!({
                "type": "CNAME",
                "name": hostname,
                "content": format!("{tunnel_id}.cfargotunnel.com"),
                "proxied": true,
                "comment": "dowiz hub"
            })),
        )
        .await?;
        v.get("result")
            .and_then(|r| r.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| ProvisionError::Unexpected(format!("{v}")))
    }

    pub async fn destroy_tunnel(&self, tunnel_id: &str) -> Result<(), ProvisionError> {
        call(
            "DELETE",
            &format!("{}/accounts/{}/cfd_tunnel/{tunnel_id}", Self::BASE, self.account_id),
            &self.token,
            None,
        )
        .await?;
        Ok(())
    }

    /// The 1,000-tunnel cap gauge P67 §5.4 polls.
    ///
    /// Counts only tunnels that are not deleted: Cloudflare keeps deleted ones
    /// in the listing with a `deleted_at`, and counting those would report a
    /// cap breach that is not real.
    pub async fn count_tunnels(&self) -> Result<u32, ProvisionError> {
        let v = call(
            "GET",
            &format!("{}/accounts/{}/cfd_tunnel?is_deleted=false&per_page=1000", Self::BASE, self.account_id),
            &self.token,
            None,
        )
        .await?;
        Ok(v.get("result")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter(|t| t.get("deleted_at").is_none_or(Value::is_null)).count() as u32)
            .unwrap_or(0))
    }
}

/// The cloud-init a fresh hub boots with.
///
/// Deliberately small and readable: it installs nothing from a package manager
/// beyond cloudflared, drops the hub binary and a systemd unit, and starts them.
/// A provisioning script nobody can read is one nobody can audit, and this one
/// runs as root on a machine holding a venue's orders.
pub fn cloud_init(hub_binary_url: &str, connector_token: &str, port: u16) -> String {
    format!(
        r#"#cloud-config
package_update: true
packages: [ca-certificates, curl]
write_files:
  - path: /etc/systemd/system/dowiz-hub.service
    content: |
      [Unit]
      Description=dowiz hub
      After=network-online.target
      Wants=network-online.target
      [Service]
      Type=simple
      User=dowiz
      WorkingDirectory=/var/lib/dowiz
      Environment=HUB_DIR=/var/lib/dowiz/hub
      ExecStart=/usr/local/bin/dowiz-hub --hub-dir /var/lib/dowiz/hub --port {port} --bind 127.0.0.1
      Restart=always
      RestartSec=2
      # The hub holds one venue's orders and needs nothing else on the box.
      NoNewPrivileges=true
      PrivateTmp=true
      ProtectSystem=strict
      ProtectHome=true
      ReadWritePaths=/var/lib/dowiz
      [Install]
      WantedBy=multi-user.target
runcmd:
  - useradd --system --home /var/lib/dowiz --create-home dowiz || true
  - curl -fsSL {hub_binary_url} -o /usr/local/bin/dowiz-hub
  - chmod 0755 /usr/local/bin/dowiz-hub
  - install -d -o dowiz -g dowiz -m 0700 /var/lib/dowiz/hub
  - curl -fsSL https://pkg.cloudflare.com/cloudflared-stable-linux-arm64.deb -o /tmp/cf.deb
  - dpkg -i /tmp/cf.deb
  - cloudflared service install {connector_token}
  - systemctl daemon-reload
  - systemctl enable --now dowiz-hub
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No credentials means a NAMED refusal, not a request to somebody else's
    /// account and not a silent no-op.
    #[test]
    fn missing_credentials_name_themselves() {
        // SAFETY: single-threaded test; nothing else reads the environment.
        unsafe {
            std::env::remove_var("HETZNER_API_TOKEN");
            std::env::remove_var("CLOUDFLARE_API_TOKEN");
            std::env::remove_var("CLOUDFLARE_ACCOUNT_ID");
        }
        match Hetzner::from_env() {
            Err(ProvisionError::NotConfigured(v)) => assert_eq!(v, "HETZNER_API_TOKEN"),
            other => panic!("{other:?}"),
        }
        match CloudflareTunnel::from_env() {
            Err(ProvisionError::NotConfigured(v)) => assert_eq!(v, "CLOUDFLARE_API_TOKEN"),
            other => panic!("{other:?}"),
        }
        unsafe { std::env::set_var("CLOUDFLARE_API_TOKEN", "x") };
        match CloudflareTunnel::from_env() {
            Err(ProvisionError::NotConfigured(v)) => assert_eq!(v, "CLOUDFLARE_ACCOUNT_ID"),
            other => panic!("{other:?}"),
        }
        unsafe { std::env::remove_var("CLOUDFLARE_API_TOKEN") };
    }

    /// Neither adapter may print its credential. Both can create and destroy
    /// real infrastructure, and a struct reaches a log line the first time
    /// anything goes wrong.
    #[test]
    fn debug_never_prints_a_cloud_token() {
        unsafe {
            std::env::set_var("HETZNER_API_TOKEN", "hetzner-secret-value");
            std::env::set_var("CLOUDFLARE_API_TOKEN", "cf-secret-value");
            std::env::set_var("CLOUDFLARE_ACCOUNT_ID", "acct-123");
        }
        let h = format!("{:?}", Hetzner::from_env().expect("hetzner"));
        assert!(!h.contains("hetzner-secret-value"), "{h}");
        assert!(h.contains("<set>"), "{h}");
        let c = format!("{:?}", CloudflareTunnel::from_env().expect("cf"));
        assert!(!c.contains("cf-secret-value"), "{c}");
        assert!(c.contains("acct-123"), "the ACCOUNT is not a secret and is needed to debug: {c}");
        unsafe {
            std::env::remove_var("HETZNER_API_TOKEN");
            std::env::remove_var("CLOUDFLARE_API_TOKEN");
            std::env::remove_var("CLOUDFLARE_ACCOUNT_ID");
        }
    }

    /// Both providers' error shapes are read, because guessing wrong turns an
    /// actionable message into "something went wrong".
    #[test]
    fn provider_errors_are_read_not_swallowed() {
        assert_eq!(
            provider_message(br#"{"error":{"code":"forbidden","message":"token lacks permission"}}"#),
            "token lacks permission"
        );
        assert_eq!(
            provider_message(br#"{"success":false,"errors":[{"code":10000,"message":"Authentication error"}]}"#),
            "Authentication error"
        );
        assert_eq!(
            provider_message(br#"{"errors":[{"message":"a"},{"message":"b"}]}"#),
            "a; b"
        );
        // Not JSON at all -- an HTML error page from a proxy, say.
        assert!(provider_message(b"<html>502 Bad Gateway</html>").contains("502"));
        assert_eq!(provider_message(b""), "");
    }

    /// The defaults are for ONE RESTAURANT and should stay that way.
    #[test]
    fn the_default_machine_is_small_and_arm() {
        let s = ServerSpec::default();
        assert_eq!(s.server_type, "cax11", "Arm, ~4 EUR/month");
        assert_eq!(s.location, "fsn1");
        assert!(s.user_data.is_none(), "cloud-init is supplied per hub, never defaulted");
        assert!(s.ssh_key_ids.is_empty(), "no key is added unless the operator names one");
    }

    /// cloud-init runs as root on a machine holding a venue's orders. These are
    /// the lines that stop it being a liability.
    #[test]
    fn the_boot_script_hardens_the_service() {
        let ci = cloud_init("https://example.com/dowiz-hub", "cf-token-value", 8080);
        assert!(ci.starts_with("#cloud-config"), "cloud-init needs its header or it is ignored");
        // The hub does not run as root.
        assert!(ci.contains("User=dowiz"));
        assert!(ci.contains("useradd --system"));
        for hardening in ["NoNewPrivileges=true", "ProtectSystem=strict", "ProtectHome=true", "PrivateTmp=true"] {
            assert!(ci.contains(hardening), "missing {hardening}");
        }
        // It listens on LOOPBACK only: everything from outside arrives through
        // the tunnel, so a bound public port would be a second, unguarded door.
        assert!(ci.contains("--bind 127.0.0.1"), "the hub must not listen publicly");
        assert!(ci.contains("--port 8080"));
        // The store directory is the only writable path, and it is not world
        // readable -- it holds the signing key.
        assert!(ci.contains("ReadWritePaths=/var/lib/dowiz"));
        assert!(ci.contains("-m 0700 /var/lib/dowiz/hub"));
        assert!(ci.contains("cf-token-value"));
    }
}
