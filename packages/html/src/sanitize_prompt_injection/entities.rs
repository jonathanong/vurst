use regex::Regex;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::LazyLock,
};

const HTML_ENTITY_DECODE_WORK_FACTOR: usize = 64;
const MIN_HTML_ENTITY_DECODE_FULL_PASSES: usize = 2;
const MIN_HTML_ENTITY_DECODE_WORK_UNITS: usize = 1_024;
const MAX_HTML_ENTITY_DECODE_NESTED_WORK_UNITS: usize = 1_000_000;

static NAMED_HTML_ENTITIES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    entities::ENTITIES
        .iter()
        .map(|entity| (entity.entity, entity.characters))
        .collect()
});

static SECURITY_CRITICAL_CASE_INSENSITIVE_HTML_ENTITIES: &[(&str, &str)] = &[
    ("&amp;", "&"),
    ("&amp", "&"),
    ("&lt;", "<"),
    ("&lt", "<"),
    ("&gt;", ">"),
    ("&gt", ">"),
    ("&quot;", "\""),
    ("&quot", "\""),
    ("&apos;", "'"),
    ("&nbsp;", " "),
    ("&nbsp", " "),
    ("&colon;", ":"),
    ("&lsqb;", "["),
    ("&rsqb;", "]"),
    ("&lbrack;", "["),
    ("&rbrack;", "]"),
    ("&vert;", "|"),
    ("&verbar;", "|"),
    ("&sol;", "/"),
];

static SECURITY_CRITICAL_CASE_INSENSITIVE_HTML_ENTITY_MAP: LazyLock<
    HashMap<&'static str, &'static str>,
> = LazyLock::new(|| {
    SECURITY_CRITICAL_CASE_INSENSITIVE_HTML_ENTITIES
        .iter()
        .copied()
        .collect()
});

static CASE_INSENSITIVE_NAMED_HTML_ENTITIES: LazyLock<HashMap<String, &'static str>> =
    LazyLock::new(|| {
        let mut candidates = HashMap::<String, Option<&'static str>>::new();

        for entity in &entities::ENTITIES {
            match candidates.entry(entity.entity.to_ascii_lowercase()) {
                Entry::Vacant(entry) => {
                    entry.insert(Some(entity.characters));
                }
                Entry::Occupied(mut entry) => {
                    if entry
                        .get()
                        .is_some_and(|characters| characters != entity.characters)
                    {
                        entry.insert(None);
                    }
                }
            }
        }

        candidates
            .into_iter()
            .filter_map(|(entity, characters)| characters.map(|value| (entity, value)))
            .collect()
    });

static HTML_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| {
    let mut semicolonless_names = entities::ENTITIES
        .iter()
        .filter_map(|entity| {
            entity
                .entity
                .strip_prefix('&')
                .filter(|name| !name.ends_with(';'))
                .map(regex::escape)
        })
        .collect::<Vec<_>>();

    semicolonless_names
        .sort_unstable_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    semicolonless_names.dedup();

    Regex::new(&format!(
        r"&(?:(?P<named>[A-Za-z][0-9A-Za-z]*;)|(?P<decimal>#\d+;)|(?P<hex>#[xX][0-9A-Fa-f]+;)|(?P<legacy>(?i:{}))(?P<legacy_tail>[^0-9A-Za-z;]|$))",
        semicolonless_names.join("|")
    ))
    .expect("BUG: invalid HTML_ENTITY_RE")
});

fn lookup_named_html_entity(entity: &str) -> Option<&'static str> {
    if let Some(replacement) = NAMED_HTML_ENTITIES.get(entity).copied() {
        return Some(replacement);
    }

    let case_folded_entity = entity.to_ascii_lowercase();
    SECURITY_CRITICAL_CASE_INSENSITIVE_HTML_ENTITY_MAP
        .get(case_folded_entity.as_str())
        .copied()
        .or_else(|| {
            CASE_INSENSITIVE_NAMED_HTML_ENTITIES
                .get(case_folded_entity.as_str())
                .copied()
        })
}

fn decode_html_entities_once(text: &str) -> String {
    HTML_ENTITY_RE
        .replace_all(
            text,
            |caps: &regex::Captures| -> std::borrow::Cow<'static, str> {
                let m = &caps[0];
                let legacy_tail = caps.name("legacy_tail").map_or("", |tail| tail.as_str());
                let entity = if legacy_tail.is_empty() {
                    m
                } else {
                    &m[..m.len() - legacy_tail.len()]
                };

                if let Some(replacement) = lookup_named_html_entity(entity) {
                    if legacy_tail.is_empty() {
                        return std::borrow::Cow::Borrowed(replacement);
                    }
                    return std::borrow::Cow::Owned(format!("{replacement}{legacy_tail}"));
                }

                if !entity.ends_with(';') {
                    return std::borrow::Cow::Owned(m.to_string());
                }

                let inner = &entity[1..entity.len() - 1];
                if let Some(digits) = inner
                    .strip_prefix("#x")
                    .or_else(|| inner.strip_prefix("#X"))
                {
                    return std::borrow::Cow::Owned(
                        u32::from_str_radix(digits, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .map_or_else(|| m.to_string(), |c| c.to_string()),
                    );
                }

                if let Some(digits) = inner.strip_prefix('#') {
                    return std::borrow::Cow::Owned(
                        digits
                            .parse::<u32>()
                            .ok()
                            .and_then(char::from_u32)
                            .map_or_else(|| m.to_string(), |c| c.to_string()),
                    );
                }

                std::borrow::Cow::Owned(m.to_string())
            },
        )
        .into_owned()
}

fn html_entity_decode_work_budget(text: &str) -> usize {
    let nested_work_budget = text
        .len()
        .saturating_mul(HTML_ENTITY_DECODE_WORK_FACTOR)
        .clamp(
            MIN_HTML_ENTITY_DECODE_WORK_UNITS,
            MAX_HTML_ENTITY_DECODE_NESTED_WORK_UNITS,
        );
    let full_pass_budget = text
        .len()
        .saturating_mul(MIN_HTML_ENTITY_DECODE_FULL_PASSES);

    nested_work_budget.max(full_pass_budget)
}

pub(super) fn decode_html_entities<'a>(
    text: impl Into<std::borrow::Cow<'a, str>>,
) -> std::borrow::Cow<'a, str> {
    let text = text.into();
    if !text.as_bytes().contains(&b'&') {
        return text;
    }
    if !HTML_ENTITY_RE.is_match(&text) {
        return text;
    }

    let mut decoded = text.to_string();
    let mut remaining_work = html_entity_decode_work_budget(&decoded);
    let mut decoded_once = false;

    loop {
        if !HTML_ENTITY_RE.is_match(&decoded) {
            return std::borrow::Cow::Owned(decoded);
        }

        let pass_work = decoded.len().max(1);
        if decoded_once && remaining_work < pass_work {
            return std::borrow::Cow::Owned(
                HTML_ENTITY_RE
                    .replace_all(&decoded, |caps: &regex::Captures| {
                        let legacy_tail = caps.name("legacy_tail").map_or("", |tail| tail.as_str());
                        format!(" {legacy_tail}")
                    })
                    .into_owned(),
            );
        }
        remaining_work = remaining_work.saturating_sub(pass_work);

        let next = decode_html_entities_once(&decoded);
        if next == decoded {
            return std::borrow::Cow::Owned(decoded);
        }
        decoded_once = true;
        decoded = next;
    }
}
