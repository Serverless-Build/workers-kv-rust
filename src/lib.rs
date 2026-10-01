use worker::*;
use serde_json::{json, Value};

const MARKER: &str = "SERVERLESS_BUILD_KV_CONFIG_RUST_V1";

#[event(fetch)]
async fn fetch(request: Request, env: Env, _context: Context) -> Result<Response> {
    let url = request.url()?;
    if request.method() != Method::Get {
        let headers = Headers::new();
        headers.set("content-type", "application/json")?;
        headers.set("allow", "GET")?;
        return Ok(Response::from_json(&json!({"error": "This public configuration demo is read-only. Seed changes through Wrangler."}))?.with_status(405).with_headers(headers));
    }
    if url.path() == "/health" { return Response::from_json(&json!({"ok": true, "marker": MARKER})); }
    if url.path() == "/" { return Response::from_json(&json!({"marker": MARKER, "pattern": "Read-heavy application configuration in Workers KV",
        "versions": ["v1", "v2"], "endpoints": ["GET /config?version=v1", "GET /evaluate?version=v1&plan=pro&region=EU", "GET /health"],
        "notes": "Configuration is seeded by the operator and eventually consistent. Feature flags are not authorization."})); }
    if !["/config", "/evaluate"].contains(&url.path()) { return Ok(Response::from_json(&json!({"error": "Not found"}))?.with_status(404)); }
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    let version = pairs.get("version").map(String::as_str).unwrap_or("v1");
    let plan = pairs.get("plan").map(String::as_str).unwrap_or("free");
    let region = pairs.get("region").map(String::as_str).unwrap_or("EU");
    if !["v1", "v2"].contains(&version) || !["free", "pro", "enterprise"].contains(&plan) || !["NA", "EU", "APAC"].contains(&region) {
        return Ok(Response::from_json(&json!({"error": "Choose version v1/v2, plan free/pro/enterprise, and region NA/EU/APAC"}))?.with_status(400));
    }
    let key = format!("app-config:{version}");
    let (config, metadata) = env.kv("KV")?.get(&key).cache_ttl(60).json_with_metadata::<Value, Value>().await?;
    let Some(config) = config else { return Ok(Response::from_json(&json!({"error": "Configuration is not seeded yet. Run npm run seed:local or seed:remote.", "key": key}))?.with_status(503)); };
    let mut body = json!({"marker": MARKER, "key": key, "revision": config["revision"], "metadata": metadata, "cacheTtlSeconds": 60,
        "consistency": "Eventually consistent; cacheTtl is a configured policy, not a measured hit or propagation time."});
    if url.path() == "/config" { body["config"] = config; }
    else {
        let enabled = config["fastSearch"]["enabled"].as_bool().unwrap_or(false)
            && config["fastSearch"]["plans"].as_array().is_some_and(|plans| plans.contains(&json!(plan)))
            && config["fastSearch"]["regions"].as_array().is_some_and(|regions| regions.contains(&json!(region)));
        body["demoInputs"] = json!({"plan": plan, "region": region});
        body["fastSearchEnabled"] = json!(enabled);
        body["note"] = json!("Caller-selected plan and region illustrate a UI feature rule; they do not grant access to protected resources.");
    }
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    headers.set("cache-control", "no-store")?;
    headers.set("x-content-type-options", "nosniff")?;
    Ok(Response::from_json(&body)?.with_headers(headers))
}
