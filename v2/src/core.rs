//! Source-derived projection. No network, host effects, script execution or mining advice.
use serde_json::{json, Value};

pub const MAX_ROWS: usize = 512;
pub const MAX_TEXT: usize = 4096;
pub const MAX_ARGUMENT: usize = 4096;
pub const COIN_FIELDS: &[(&str, &str)] = &[
    ("Coin", "coin"),
    ("Name", "name"),
    ("Algo", "algo"),
    ("", ""),
    ("USD Price", "usdPrice"),
    ("USD Price 1h", "usdPrice1h"),
    ("USD Price 24h", "usdPrice24h"),
    ("USD Price 7d", "usdPrice7d"),
    ("USD Price 30d", "usdPrice30d"),
    ("", ""),
    ("Coin Estimate", "coinEstimate"),
    ("Coin EstimateUnit", "coinEstimateUnit"),
    ("Coin Estimate1h", "coinEstimate1h"),
    ("Coin Estimate24h", "coinEstimate24h"),
    ("Coin Estimate7d", "coinEstimate7d"),
    ("Coin Estimate30d", "coinEstimate30d"),
    ("Coin EstimateAvg30d", "coinEstimateAvg30d"),
    ("USD Estimate", "usdEstimate"),
    ("USD Estimate1h", "usdEstimate1h"),
    ("USD Estimate24h", "usdEstimate24h"),
    ("USD Estimate7d", "usdEstimate7d"),
    ("USD Estimate30d", "usdEstimate30d"),
    ("", ""),
    ("Network Hashrate", "networkHashrate"),
    ("Network Difficulty", "networkDifficulty"),
    ("Emission", "emission"),
    ("EmissionUSD", "emissionUSD"),
];

fn js_space(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
/// ECMAScript StringNumericLiteral, including unsigned radix prefixes and -0.
pub fn number(s: &str) -> f64 {
    let s = s.trim_matches(js_space);
    if s.is_empty() {
        return 0.0;
    }
    if matches!(s, "Infinity" | "+Infinity") {
        return f64::INFINITY;
    }
    if s == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    for (prefix, base) in [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
    ] {
        if let Some(digits) = s.strip_prefix(prefix) {
            if digits.is_empty() {
                return f64::NAN;
            }
            // Round radix integers once using a bit string (rather than repeated f64 multiply).
            let mut bits = String::new();
            for ch in digits.chars() {
                let Some(d) = ch.to_digit(base) else {
                    return f64::NAN;
                };
                let width = match base {
                    16 => 4,
                    8 => 3,
                    _ => 1,
                };
                bits.push_str(&format!("{d:0width$b}"));
            }
            let bits = bits.trim_start_matches('0');
            if bits.is_empty() {
                return 0.0;
            }
            let take = bits.len().min(53);
            let mut significand = u64::from_str_radix(&bits[..take], 2).unwrap();
            if bits.len() > 53
                && bits.as_bytes()[53] == b'1'
                && (bits[54..].contains('1') || significand & 1 == 1)
            {
                significand += 1;
            }
            return (significand as f64) * 2.0_f64.powi((bits.len() - take).min(2048) as i32);
        }
    }
    let bytes = s.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - start;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - start;
    }
    if digits == 0 {
        return f64::NAN;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if start == i {
            return f64::NAN;
        }
    }
    if i != bytes.len() {
        return f64::NAN;
    }
    s.parse().unwrap_or(f64::NAN)
}
pub fn validate_power(s: &str) -> Result<(), String> {
    if s.len() > MAX_ARGUMENT {
        return Err("Power Cost exceeds the 4096-byte resource limit".into());
    }
    let n = number(s);
    if n == 0.0 || n.is_nan() {
        return Err("Power Cost must be a number!".into());
    }
    if !(0.0..=1.0).contains(&n) {
        return Err("Power Cost must be between 0 and 1.00!".into());
    }
    Ok(())
}
pub fn js_string(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => ryu_js::Buffer::new()
            .format(n.as_f64().unwrap_or(f64::NAN))
            .into(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else {
                    js_string(Some(v))
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}
fn to_number(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => number(s),
        Some(Value::Array(_)) => number(&js_string(v)),
        _ => f64::NAN,
    }
}
/// Intl.NumberFormat("en", {currency:"USD",style:"currency"}): decimal
/// half-expand on the shortest JS number representation, not binary `round()`.
pub fn usd(v: Option<&Value>) -> String {
    let n = to_number(v);
    if n.is_nan() {
        return "$NaN".into();
    }
    let sign = if n.is_sign_negative() { "-" } else { "" };
    if n.is_infinite() {
        return format!("{sign}$∞");
    }
    let text = ryu_js::Buffer::new().format(n.abs()).to_owned();
    let (mantissa, exponent) = text
        .split_once('e')
        .map_or((text.as_str(), 0), |(m, e)| (m, e.parse::<i32>().unwrap()));
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as i32 + exponent;
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let mut cents = String::new();
    for pos in 0..(decimal + 2).max(1) {
        cents.push(if pos < decimal + 2 {
            digits.as_bytes().get(pos as usize).copied().unwrap_or(b'0') as char
        } else {
            '0'
        });
    }
    if decimal < 0 {
        cents = "0".repeat(2);
    }
    if decimal == -1 {
        cents = format!("0{}", digits.as_bytes()[0] as char);
    }
    if decimal == 0 {
        cents = format!(
            "{}{}",
            digits.as_bytes()[0] as char,
            digits.as_bytes().get(1).copied().unwrap_or(b'0') as char
        );
    }
    let round_pos = decimal + 2;
    let round = if round_pos >= 0 {
        digits
            .as_bytes()
            .get(round_pos as usize)
            .copied()
            .unwrap_or(b'0')
            >= b'5'
    } else {
        false
    };
    let mut cents = cents.into_bytes();
    if round {
        let mut carry = true;
        for digit in cents.iter_mut().rev() {
            if *digit < b'9' {
                *digit += 1;
                carry = false;
                break;
            }
            *digit = b'0';
        }
        if carry {
            cents.insert(0, b'1');
        }
    }
    while cents.len() < 3 {
        cents.insert(0, b'0');
    }
    let cents = String::from_utf8(cents).unwrap();
    let (integer, fraction) = cents.split_at(cents.len() - 2);
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let mut grouped = String::new();
    for (i, c) in integer.chars().enumerate() {
        if i != 0 && (integer.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{sign}${grouped}.{fraction}")
}
pub fn node(kind: &str, props: Value, children: Vec<Value>) -> Value {
    json!({"type":kind,"props":props,"children":children})
}
fn label(title: &str, text: Option<&Value>) -> Value {
    let mut p = json!({"title":title});
    if let Some(text) = text {
        p["text"] = text.clone();
    }
    node("List.Item.Detail.Metadata.Label", p, vec![])
}
fn separator() -> Value {
    node("List.Item.Detail.Metadata.Separator", json!({}), vec![])
}
fn checked_text(v: Option<&Value>) -> Result<(), String> {
    if v.is_some_and(|v| !(v.is_null() || v.is_string() || v.is_number() || v.is_boolean()))
        || js_string(v).len() > MAX_TEXT
    {
        return Err("Service response has invalid or oversized text fields".into());
    }
    Ok(())
}
/// Keep only source-consumed fields. Never retain arbitrary service payload/URLs.
pub fn normalize(command: &str, data: Value, secret: &str) -> Result<Value, String> {
    if data.is_object() && data.get("title").is_some() {
        return Err("Hashrate API reported an error".into());
    }
    let mut output = Vec::new();
    if command == "coins" {
        let list = data
            .as_array()
            .ok_or("Expected a coins array from Hashrate API")?;
        if list.len() > MAX_ROWS {
            return Err("Service response exceeds the 512-row resource limit".into());
        }
        for coin in list {
            if !coin.is_object() {
                return Err("Invalid coin record from Hashrate API".into());
            }
            let mut normalized = json!({});
            for (_, key) in COIN_FIELDS.iter().filter(|(_, k)| !k.is_empty()) {
                checked_text(coin.get(key))?;
                if let Some(v) = coin.get(key) {
                    normalized[key] = v.clone();
                }
            }
            output.push(normalized);
        }
    } else {
        let map = data
            .as_object()
            .ok_or("Expected GPU estimates object from Hashrate API")?;
        if map.len() > MAX_ROWS {
            return Err("Service response exceeds the 512-row resource limit".into());
        }
        // JS Object.entries orders canonical uint32 indices first, then insertion order.
        let mut pairs: Vec<_> = map.iter().collect();
        pairs.sort_by_key(|(k, _)| {
            k.parse::<u32>()
                .ok()
                .filter(|n| *n != u32::MAX && n.to_string() == **k)
                .map_or((1, 0), |n| (0, n))
        });
        for (key, estimate) in pairs {
            if key.len() > MAX_TEXT {
                return Err("GPU key exceeds the resource limit".into());
            }
            let mut row = json!({"id":key,"estimate":{}});
            for (group, fields) in [
                ("device", &["name", "brand"][..]),
                ("profit", &["coin", "revenueUSD", "profitUSD"][..]),
                ("revenue", &["coin", "revenueUSD", "profitUSD"][..]),
                ("profit24", &["coin", "revenueUSD24", "profitUSD24"][..]),
                ("revenue24", &["coin", "revenueUSD24", "profitUSD24"][..]),
            ] {
                let g = estimate
                    .get(group)
                    .filter(|g| g.is_object())
                    .ok_or("Invalid GPU record from Hashrate API")?;
                row["estimate"][group] = json!({});
                for field in fields {
                    checked_text(g.get(field))?;
                    if let Some(v) = g.get(field) {
                        row["estimate"][group][field] = v.clone();
                    }
                }
            }
            output.push(row);
        }
    }
    let output = json!(output);
    let serialized = serde_json::to_string(&output).map_err(|_| "Response encoding failed")?;
    if !secret.is_empty()
        && (serialized.contains(secret)
            || serialized.contains(&tezbar_extension_sdk::encode_form_query(secret)))
    {
        return Err("Service response contains private request data".into());
    }
    Ok(output)
}
pub fn rows(
    command: &str,
    data: &Value,
    detail: bool,
    action_ids: &[String],
) -> Result<Vec<Value>, String> {
    let data = data.as_array().ok_or("Invalid cache data")?;
    let mut rows = Vec::new();
    for (index, item) in data.iter().enumerate() {
        let (id, title, subtitle, accessories, metadata) = if command == "coins" {
            let title = format!(
                "{}{}",
                js_string(item.get("coin")),
                if detail {
                    String::new()
                } else {
                    format!(" - {}", js_string(item.get("name")))
                }
            );
            let metadata = COIN_FIELDS
                .iter()
                .map(|(title, key)| {
                    if key.is_empty() {
                        separator()
                    } else {
                        label(title, item.get(key))
                    }
                })
                .collect();
            (
                format!("coin:{index}"),
                title,
                item["algo"].clone(),
                json!([{"text":format!("${}",js_string(item.get("usdPrice")))}]),
                metadata,
            )
        } else {
            let estimate = &item["estimate"];
            let mut metadata = Vec::new();
            for (i, (heading, group, suffix)) in [
                ("Highest Profit", "profit", ""),
                ("Highest Revenue", "revenue", ""),
                ("Highest Profit 24H", "profit24", "24"),
                ("Highest Revenue 24H", "revenue24", "24"),
            ]
            .iter()
            .enumerate()
            {
                if i != 0 {
                    metadata.push(separator());
                }
                metadata.push(label(heading, None));
                metadata.push(label("Coin", estimate[group].get("coin")));
                metadata.push(label(
                    "Revenue",
                    Some(&json!(usd(
                        estimate[group].get(format!("revenueUSD{suffix}").as_str())
                    ))),
                ));
                metadata.push(label(
                    "Profit",
                    Some(&json!(usd(
                        estimate[group].get(format!("profitUSD{suffix}").as_str())
                    ))),
                ));
            }
            (
                format!("gpu:{index}"),
                estimate["device"]["name"]
                    .as_str()
                    .unwrap_or("Untitled")
                    .into(),
                estimate["device"]["brand"].clone(),
                json!([]),
                metadata,
            )
        };
        rows.push(node("List.Item",json!({"id":id,"title":title,"subtitle":subtitle,"icon":if command=="coins" {"Icon.Coin"} else {"Icon.Dot"},"accessories":accessories,"actionIds":action_ids,
            "detail":node("List.Item.Detail",json!({}),vec![node("List.Item.Detail.Metadata",json!({}),metadata)])}),vec![]));
    }
    Ok(rows)
}

/// Locked utils fetch-utils.ts media parser, including its broad JSON word match.
pub fn is_json_content_type(header: &str) -> bool {
    let media = header
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let Some((kind, subtype)) = media.split_once('/') else {
        return false;
    };
    let valid = |s: &str, sub: bool| {
        !s.is_empty()
            && s.len() <= 127
            && s.as_bytes()[0].is_ascii_alphanumeric()
            && s.bytes().all(|b| {
                b.is_ascii_alphanumeric() || b"!#$&^_-".contains(&b) || (sub && b".+".contains(&b))
            })
    };
    if !valid(kind, false) || !valid(subtype, true) {
        return false;
    }
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let json_word = |s: &str| {
        s.match_indices("json").any(|(i, _)| {
            (i == 0 || !word(s.as_bytes()[i - 1]))
                && (i + 4 == s.len() || !word(s.as_bytes()[i + 4]))
        })
    };
    let (subtype, suffix) = subtype
        .rsplit_once('+')
        .map_or((subtype, ""), |(s, x)| (s, x));
    json_word(subtype) || json_word(suffix)
}
