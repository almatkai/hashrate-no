pub mod core;
use chrono::Local;
use core::{node, normalize, rows, validate_power};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tezbar_extension_sdk::{unchanged, view, BoxFuture, CoreResult, Extension, ExtensionContext};
use tokio::sync::Mutex;

pub const ID: &str = "com.tezbar.native.hashrate-no";
pub const VERSION: &str = "2.0.0";
pub const MAX_BODY: usize = 512 * 1024;
pub const MAX_CACHE: usize = 8 * 1024 * 1024;
pub const MAX_VIEW: usize = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 16;
const MAX_OWNERS: usize = 128;
const MAX_KEY: usize = 4096;
const ORIGIN: &str = "https://api.hashrate.no";

pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
pub fn cache_key(secret: &str, command: &str, power: &str) -> String {
    digest(&format!("{}:{command}:{}", digest(secret), digest(power)))
}
fn local_date(timestamp: Option<i64>, fallback: &str) -> String {
    timestamp
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|instant| {
            instant
                .with_timezone(&Local)
                .format("%-m/%-d/%Y")
                .to_string()
        })
        .unwrap_or_else(|| fallback.to_owned())
}
fn client(
    origin: &str,
    certificate: Option<reqwest::Certificate>,
) -> Result<reqwest::Client, String> {
    let expected = reqwest::Url::parse(origin).map_err(|_| "Invalid service origin")?;
    let mut builder = reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            let url = attempt.url();
            if attempt.previous().len() >= 3
                || url.origin() != expected.origin()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                attempt.error("Service redirect rejected")
            } else {
                attempt.follow()
            }
        }));
    if let Some(certificate) = certificate {
        builder = builder.add_root_certificate(certificate);
    }
    builder
        .build()
        .map_err(|_| "Could not initialize HTTPS client".into())
}
struct Owner {
    context: ExtensionContext,
    ids: [String; 2],
    serial: u64,
}
#[derive(Default)]
struct Cache {
    value: Option<Value>,
    completed: HashMap<String, u64>,
}
pub struct Adapter {
    client: reqwest::Client,
    origin: String,
    cache: Mutex<Cache>,
    owners: Mutex<HashMap<String, Owner>>,
    serial: AtomicU64,
}
async fn is_cancelled(ctx: &ExtensionContext) -> bool {
    tokio::select! { biased; _ = ctx.cancelled() => true, _ = std::future::ready(()) => false }
}
impl Adapter {
    pub fn production() -> Result<Self, String> {
        Self::new(ORIGIN, None)
    }
    fn new(origin: &str, cert: Option<reqwest::Certificate>) -> Result<Self, String> {
        Ok(Self {
            client: client(origin, cert)?,
            origin: origin.into(),
            cache: Mutex::default(),
            owners: Mutex::default(),
            serial: AtomicU64::new(1),
        })
    }
    /// Compiled only into an explicitly built QA binary, never the production artifact.
    #[cfg(feature = "qa")]
    pub fn owned_tls_fixture(port: u16, pem: &[u8]) -> Result<Self, String> {
        let cert =
            reqwest::Certificate::from_pem(pem).map_err(|_| "Invalid owned TLS certificate")?;
        Self::new(&format!("https://localhost:{port}"), Some(cert))
    }
    async fn fetch(&self, command: &str, secret: &str, power: &str) -> CoreResult {
        let endpoint = if command == "coins" {
            "coins"
        } else {
            "gpuEstimates"
        };
        let mut url = reqwest::Url::parse(&format!("{}/v1/{endpoint}", self.origin))
            .map_err(|_| "Invalid fixed service URL")?;
        url.query_pairs_mut().append_pair("apiKey", secret);
        if command != "coins" {
            url.query_pairs_mut().append_pair("powerCost", power);
        }
        let mut response = self.client.get(url).send().await.map_err(|_| "Hashrate HTTPS request failed (timeout, connection, TLS or redirect); use Refresh to retry")?;
        if !response.status().is_success() {
            return Err(format!(
                "Hashrate API returned HTTP {}; use Refresh to retry",
                response.status().as_u16()
            ));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        if !core::is_json_content_type(content_type) {
            return Err("Hashrate API did not return JSON".into());
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BODY as u64)
        {
            return Err("Service response exceeds the 512 KiB resource limit".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Hashrate response read failed; use Refresh to retry")?
        {
            if bytes.len() + chunk.len() > MAX_BODY {
                return Err("Service response exceeds the 512 KiB resource limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let raw =
            serde_json::from_slice(&bytes).map_err(|_| "Hashrate API returned invalid JSON")?;
        normalize(command, raw, secret)
    }
    async fn prune(&self) {
        let mut owners = self.owners.lock().await;
        let ids: Vec<_> = owners.keys().cloned().collect();
        for id in ids {
            if is_cancelled(&owners[&id].context).await {
                owners.remove(&id);
            }
        }
    }
    async fn consume(&self, session: &str, action: &str) -> Option<usize> {
        let mut owners = self.owners.lock().await;
        let owner = owners.get_mut(session)?;
        let index = owner
            .ids
            .iter()
            .position(|id| !id.is_empty() && id == action)?;
        // Both IDs become inert before any await/network/storage. SDK rollback cannot replay them.
        owner.ids = [String::new(), String::new()];
        Some(index)
    }
    async fn render(
        &self,
        ctx: &ExtensionContext,
        command: &str,
        payload: &Value,
        state: &mut Value,
        rotate: bool,
    ) -> CoreResult {
        let session = payload["sessionId"]
            .as_str()
            .ok_or("Missing session owner")?;
        if is_cancelled(ctx).await {
            return Err("Operation cancelled".into());
        }
        let serial = self.serial.fetch_add(1, Ordering::SeqCst);
        let mut ids = [
            format!("hashrate:{serial}:details"),
            format!("hashrate:{serial}:refresh"),
        ];
        if !rotate {
            if let Some(owner) = self.owners.lock().await.get(session) {
                if owner.ids.iter().all(|id| !id.is_empty()) {
                    ids = owner.ids.clone();
                }
            }
        }
        let detail = state["detail"].as_bool().unwrap_or(command != "coins");
        let data = state["data"].as_array().ok_or("Invalid session data")?;
        let power = payload["argumentValues"]["power_cost"]
            .as_str()
            .unwrap_or("");
        let displayed_date = local_date(
            state["updated"].as_i64(),
            state["date"].as_str().unwrap_or(""),
        );
        let section = if command == "coins" {
            format!("{} coins | last_update: {}", data.len(), displayed_date)
        } else {
            format!(
                "{} estimates | cost: {power} | last_update: {}",
                data.len(),
                displayed_date
            )
        };
        let mut projected = rows(command, &state["data"], detail, &ids)?;
        // Pinned ListRuntime infers detail visibility from row details, ignoring
        // isShowingDetail. Omit the pane while hidden; retain all data in state.
        if !detail {
            for row in &mut projected {
                row["props"].as_object_mut().unwrap().shift_remove("detail");
            }
        }
        let mut children = vec![node("List.Section", json!({"title":section}), projected)];
        if data.is_empty() {
            children.push(node("List.EmptyView",json!({"title":state["error"].as_str().unwrap_or(if command == "coins" {"No coins"} else {"No GPU estimates"}),"description":"Use Refresh to request Hashrate data."}),vec![]));
        }
        let root = node(
            "List",
            json!({"isLoading":false,"isShowingDetail":detail,"filtering":true,"searchBarPlaceholder":if command=="coins" {"Search coins"} else {"Search GPUs"}}),
            children,
        );
        let mut result = view(
            if command == "coins" {
                "Coins"
            } else {
                "GPU Estimates"
            },
            root,
            vec![
                json!({"id":ids[0],"title":"Toggle Details","kind":"action"}),
                json!({"id":ids[1],"title":"Refresh","kind":"action"}),
            ],
        );
        result["refreshMs"] = Value::Null;
        if let Some(error) = state["error"].as_str() {
            result["message"] = json!(error);
        }
        if serde_json::to_vec(&result)
            .map_err(|_| "View encoding failed")?
            .len()
            > MAX_VIEW
        {
            return Err("View exceeds the 8 MiB resource limit".into());
        }
        let mut owners = self.owners.lock().await;
        if !owners.contains_key(session) && owners.len() >= MAX_OWNERS {
            // Cancelled opens need not receive dispose. Bound abandoned owners even
            // if cancellation raced the successful final synchronous render.
            if let Some(oldest) = owners
                .iter()
                .min_by_key(|(_, o)| o.serial)
                .map(|(id, _)| id.clone())
            {
                owners.remove(&oldest);
            }
        }
        owners.insert(
            session.into(),
            Owner {
                context: ctx.clone(),
                ids,
                serial,
            },
        );
        Ok(result)
    }
    async fn snapshot(
        &self,
        ctx: &ExtensionContext,
        command: &str,
        payload: &Value,
        state: &mut Value,
        refresh: bool,
        toggle: bool,
    ) -> Result<(), String> {
        let secret = payload["preferences"]["api_key"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or(
                "API Key is required; save it in extension preferences and reopen the command",
            )?;
        if secret.len() > MAX_KEY {
            return Err("API Key exceeds the 4096-byte resource limit".into());
        }
        let power = if command == "coins" {
            ""
        } else {
            payload["argumentValues"]["power_cost"]
                .as_str()
                .unwrap_or("")
        };
        let account = digest(secret);
        let key = cache_key(secret, command, power);
        let arg_hash = digest(power);
        // Source refresh() records its Date when invoked, before awaiting I/O.
        let when = Local::now();
        let started = self.serial.fetch_add(1, Ordering::SeqCst);
        let mut cache = self.cache.lock().await;
        if cache.value.is_none() {
            cache.value = Some(read_cache(&ctx.paths.data_dir)?);
        }
        let newer_completed = cache.completed.get(&key).copied().unwrap_or(0) >= started;
        let mut completed_serial = None;
        let root = cache.value.as_mut().unwrap();
        let existing = root["entries"][&key].clone();
        let length = existing["data"].as_array().map_or(0, Vec::len);
        let settings_key = format!("{account}:{command}");
        let mut settings = root["settings"][&settings_key].clone();
        if !settings.is_object() {
            settings = json!({"detail": command != "coins"});
        }
        if toggle {
            settings["detail"] = json!(!state["detail"].as_bool().unwrap_or(command != "coins"));
        }
        let changed_cost =
            command != "coins" && length > 0 && settings["lastPowerCost"] != arg_hash;
        // Source sets lastPowerCost only on a nonempty cache with a changed input.
        if changed_cost {
            settings["lastPowerCost"] = json!(arg_hash);
        }
        let fetch = refresh
            || (state.is_null() || state.as_object().is_some_and(|o| o.is_empty()))
                && (length == 0 || changed_cost);
        let mut entry = existing;
        // Completed request checkpoints deduplicate concurrently queued requests.
        if fetch && !newer_completed {
            let date = when.format("%-m/%-d/%Y").to_string();
            let result = self.fetch(command, secret, power).await;
            completed_serial = Some(self.serial.fetch_add(1, Ordering::SeqCst));
            entry = match result {
                Ok(data) => {
                    json!({"data":data,"date":date,"updated":when.timestamp_millis()})
                }
                Err(error) => {
                    let data = entry["data"]
                        .as_array()
                        .map(|a| json!(a))
                        .unwrap_or(json!([]));
                    json!({"data":data,"date":date,"error":error,"updated":when.timestamp_millis()})
                }
            };
            // Finished failures are shown until a deliberate Refresh; no auto retry.
            root["entries"][&key] = entry.clone();
            settings["date"] = entry["date"].clone();
            settings["updated"] = entry["updated"].clone();
        }
        root["settings"][&settings_key] = settings.clone();
        trim_map(&mut root["entries"], MAX_ENTRIES);
        trim_map(&mut root["settings"], MAX_ENTRIES * 2);
        if entry.is_null() {
            entry = json!({"data":[],"date":Local::now().format("%-m/%-d/%Y").to_string()});
        }
        *state = json!({"data":entry["data"],"date":settings.get("date").unwrap_or(&entry["date"]),"updated":settings.get("updated").unwrap_or(&entry["updated"]),"error":entry["error"],"detail":settings["detail"]});
        // Synchronous atomic replacement has no cancellation point between completed
        // network result and checkpoint. Neither key nor request URL enters the file.
        let persisted = write_cache(&ctx.paths.data_dir, root);
        if let Some(serial) = completed_serial {
            cache.completed.insert(key, serial);
            if cache.completed.len() > MAX_ENTRIES {
                if let Some(oldest) = cache
                    .completed
                    .iter()
                    .min_by_key(|(_, v)| **v)
                    .map(|(k, _)| k.clone())
                {
                    cache.completed.remove(&oldest);
                }
            }
        }
        if persisted.is_err() {
            state["error"] =
                json!("Hashrate result is available in memory; private cache could not be saved");
        }
        Ok(())
    }
}
fn trim_map(v: &mut Value, limit: usize) {
    if let Some(map) = v.as_object_mut() {
        while map.len() > limit {
            let oldest = map.keys().next().cloned().unwrap();
            map.shift_remove(&oldest);
        }
    }
}
fn read_cache(dir: &Path) -> Result<Value, String> {
    let path = dir.join("hashrate-cache.json");
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"version":1,"entries":{},"settings":{}}))
        }
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() <= MAX_CACHE as u64 => {}
        _ => return Err("Private cache is unreadable, unsafe or exceeds 8 MiB".into()),
    }
    let mut b = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "Private cache could not be read")?
        .take((MAX_CACHE + 1) as u64)
        .read_to_end(&mut b)
        .map_err(|_| "Private cache could not be read")?;
    if b.len() > MAX_CACHE {
        return Err("Private cache exceeds 8 MiB".into());
    }
    let v: Value = serde_json::from_slice(&b).map_err(|_| "Private cache JSON is invalid")?;
    if v["version"] != 1
        || !v["entries"].is_object()
        || !v["settings"].is_object()
        || v["entries"].as_object().unwrap().len() > MAX_ENTRIES
        || v["settings"].as_object().unwrap().len() > MAX_ENTRIES * 2
    {
        return Err("Private cache schema is invalid".into());
    }
    // Revalidate cached projected records before rendering (no paths/URLs accepted).
    for entry in v["entries"].as_object().unwrap().values() {
        let data = entry["data"]
            .as_array()
            .ok_or("Private cache data is invalid")?;
        if data.len() > core::MAX_ROWS || entry["date"].as_str().is_none_or(|s| s.len() > 128) {
            return Err("Private cache resource bounds exceeded".into());
        }
        if entry
            .get("error")
            .is_some_and(|e| !e.is_null() && e.as_str().is_none_or(|s| s.len() > 256))
        {
            return Err("Private cache error is invalid".into());
        }
    }
    Ok(v)
}
fn write_cache(dir: &Path, value: &Value) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| "Could not create private cache directory")?;
    let metadata = std::fs::symlink_metadata(dir).map_err(|_| "Private directory unavailable")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Private directory is unsafe".into());
    }
    let path = dir.join("hashrate-cache.json");
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err("Private cache is unsafe".into());
    }
    let bytes = serde_json::to_vec(value).map_err(|_| "Private cache encoding failed")?;
    if bytes.len() > MAX_CACHE {
        return Err("Private cache exceeds 8 MiB".into());
    }
    let mut file =
        tempfile::NamedTempFile::new_in(dir).map_err(|_| "Private cache write failed")?;
    file.write_all(&bytes)
        .map_err(|_| "Private cache write failed")?;
    file.persist(path)
        .map_err(|_| "Private cache replacement failed")?;
    Ok(())
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        ID
    }
    fn version(&self) -> &'static str {
        VERSION
    }
    fn commands(&self) -> &'static [&'static str] {
        &["coins", "gpu_estimates"]
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            self.prune().await;
            if operation == "extension:dispose-session" {
                if let Some(s) = payload["sessionId"].as_str() {
                    self.owners.lock().await.remove(s);
                }
                return Ok(unchanged());
            }
            if operation == "extension:refresh-session" && state.get("data").is_some() {
                let session = payload["sessionId"].as_str().unwrap_or("");
                let needs_actions = self
                    .owners
                    .lock()
                    .await
                    .get(session)
                    .is_none_or(|owner| owner.ids.iter().all(String::is_empty));
                if needs_actions {
                    return self.render(ctx, command, payload, state, true).await;
                }
            }
            if operation == "extension:search-text-changed" && state.get("data").is_some() {
                if payload["searchText"]
                    .as_str()
                    .is_some_and(|text| text.len() > 4096)
                {
                    return Err("Search exceeds the 4096-byte resource limit".into());
                }
                // The pinned host debounces callbacks even for local filtering and
                // expects a view. Never echo searchText: the renderer owns its draft.
                return self.render(ctx, command, payload, state, false).await;
            }
            if matches!(
                operation,
                "extension:refresh-session" | "extension:load-more"
            ) {
                return Ok(unchanged());
            }
            if !self.commands().contains(&command) {
                return Err("Unknown command".into());
            }
            if command == "gpu_estimates" {
                let power = payload["argumentValues"]["power_cost"]
                    .as_str()
                    .unwrap_or("");
                if let Err(error) = validate_power(power) {
                    return Ok(tezbar_extension_sdk::no_view(&error));
                }
            }
            let (refresh, toggle) = if operation == "extension:run-command" {
                (false, false)
            } else if operation == "extension:invoke-action" {
                let session = payload["sessionId"].as_str().ok_or("Missing session")?;
                let action = payload["actionId"].as_str().ok_or("Missing action ID")?;
                match self.consume(session, action).await {
                    Some(0) => (false, true),
                    Some(1) => (true, false),
                    _ => return self.render(ctx, command, payload, state, false).await,
                }
            } else {
                return Err("Unsupported operation".into());
            };
            self.snapshot(ctx, command, payload, state, refresh, toggle)
                .await?;
            self.render(ctx, command, payload, state, true).await
        })
    }
}

#[cfg(test)]
mod tests;
