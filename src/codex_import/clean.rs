//! Cleanup of wiki-scraped text before it is stored in the codex tables.
//!
//! The upstream wiki API returns two very different shapes:
//!
//! * Multi-line text with recognizable section headers — handled by the
//!   legacy line-based path (`clean_wiki_text`).
//! * A **single line** gluing together the infobox caption, labeled fields
//!   ("Aliases Muad'Dib, Usul", "Born 10176 AG Place of birth Caladan"),
//!   a "Behind the Scenes" block with actor names, and the biography with
//!   paragraph breaks lost ("...Leto Atreides II.House Atreides was...").
//!   That shape is parsed by `parse_single_line`, which extracts the
//!   profile fields into a compact `## Profile` block, drops the noise,
//!   and reflows the biography into readable paragraphs.
//!
//! All cleanup lives here (not in one-off data fixes) so every future
//! re-import benefits from it.

// ---------------------------------------------------------------------------
// Legacy line-based path
// ---------------------------------------------------------------------------

/// Sections that are removed entirely (content skipped until next header).
const DROP_SECTIONS: &[&str] = &[
    "behind the scenes",
    "references",
    "external links",
    "see also",
    "notes",
    "gallery",
];

/// Sections that are kept but normalized into standalone headers.
const KEEP_SECTIONS: &[&str] = &[
    "titles",
    "affiliation",
    "affiliations",
    "biography",
    "history",
    "early life",
    "later life",
    "physical characteristics",
    "physical description",
    "characteristics",
    "appearance",
    "appearances",
    "personality",
    "personality and traits",
    "skills",
    "abilities",
    "skills and abilities",
    "education",
    "relationships",
    "family",
    "etymology",
    "equipment",
    "weapons",
    "legacy",
    "role",
    "description",
    "overview",
];

/// Parenthetical fragments that are pure scraping noise.
const PAREN_KEYWORDS: &[&str] = &[
    "fanart",
    "fan art",
    "art by",
    "artist",
    "miniseries",
    "film",
    "movie",
];

#[derive(Debug, PartialEq)]
enum LineKind {
    Prose,
    Header(String),
    Drop,
}

/// Cleans a raw wiki description: drops trivial sections, strips film /
/// fan-art artifacts, separates recognized sections with blank lines and
/// marks them as `## Title` headers.
pub fn clean_wiki_text(raw: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut dropping = false;

    for raw_line in raw.lines() {
        let trimmed = raw_line.trim();

        match classify(trimmed) {
            LineKind::Header(name) => {
                dropping = false;
                push_blank(&mut out);
                out.push(format!("## {}", name));
                push_blank(&mut out);
            }
            LineKind::Drop => dropping = true,
            LineKind::Prose => {
                if dropping {
                    continue;
                }
                if trimmed.is_empty() {
                    push_blank(&mut out);
                    continue;
                }
                match split_leading_header(trimmed) {
                    Some((true, _name, _rest)) => dropping = true,
                    Some((false, name, rest)) => {
                        push_blank(&mut out);
                        out.push(format!("## {}", name));
                        push_blank(&mut out);
                        if let Some(cleaned) = clean_prose_line(rest.trim()) {
                            out.push(cleaned);
                        }
                    }
                    None => {
                        if let Some(cleaned) = clean_prose_line(trimmed) {
                            out.push(cleaned);
                        }
                    }
                }
            }
        }
    }

    collapse_blanks(out).join("\n")
}

fn classify(line: &str) -> LineKind {
    let mut candidate = line.trim().to_lowercase();
    // Strip common wiki header markers and trailing colons.
    candidate = candidate
        .trim_start_matches(['#', '=', ' '])
        .trim_end_matches(['=', ':', ' '])
        .to_string();

    if DROP_SECTIONS.contains(&candidate.as_str()) {
        return LineKind::Drop;
    }
    if let Some(kept) = KEEP_SECTIONS.iter().find(|s| **s == candidate) {
        return LineKind::Header(title_case(kept));
    }
    LineKind::Prose
}

/// Words that signal a sentence merely *starts* with a header-like word
/// instead of being a glued section header ("Titles are earned...").
const CONTINUATION_WORDS: &[&str] = &[
    "are", "is", "was", "were", "be", "been", "has", "have", "had", "of", "and", "or", "in", "on",
    "for", "to", "that", "this", "a", "an", "the",
];

/// Detects headers glued to their content by the scraper
/// (`"Biography Paul was born..."`). Returns `(is_drop_section, name, rest)`.
fn split_leading_header(line: &str) -> Option<(bool, String, &str)> {
    let mut candidates: Vec<&str> = KEEP_SECTIONS.to_vec();
    candidates.extend(DROP_SECTIONS.iter().copied());
    candidates.sort_by_key(|h| std::cmp::Reverse(h.len()));

    for header in candidates {
        if line.len() <= header.len() || !line.is_char_boundary(header.len()) {
            continue;
        }
        if !line[..header.len()].eq_ignore_ascii_case(header) {
            continue;
        }
        let next = line[header.len()..].chars().next()?;
        if next != ' ' && next != ':' {
            continue;
        }
        let rest = line[header.len()..]
            .trim_start_matches([' ', ':'])
            .trim_start();
        if let Some(first_word) = rest.split_whitespace().next() {
            if CONTINUATION_WORDS.contains(&first_word.to_lowercase().as_str()) {
                continue;
            }
        } else {
            // Nothing follows: it is a bare header after all.
            return Some((DROP_SECTIONS.contains(&header), title_case(header), ""));
        }
        return Some((DROP_SECTIONS.contains(&header), title_case(header), rest));
    }
    None
}

fn title_case(section: &str) -> String {
    section
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_blank(out: &mut Vec<String>) {
    if out.last().is_none_or(|l| !l.is_empty()) {
        out.push(String::new());
    }
}

fn collapse_blanks(lines: Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        if line.is_empty() && result.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        result.push(line);
    }
    while result.last().is_some_and(|l| l.is_empty()) {
        result.pop();
    }
    result
}

/// Removes noise from a single prose line. Returns `None` if nothing
/// meaningful remains.
fn clean_prose_line(line: &str) -> Option<String> {
    let without_parens = strip_noisy_parentheticals(line);
    let without_trivia = drop_trivial_sentences(&without_parens);
    let collapsed = collapse_spaces(without_trivia.trim());
    if collapsed.is_empty() {
        None
    } else {
        Some(collapsed)
    }
}

/// Removes `( ... )` groups whose content is fan-art / film noise.
fn strip_noisy_parentheticals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0usize;
    let mut group = String::new();

    for ch in line.chars() {
        match ch {
            '(' => {
                depth += 1;
                if depth == 1 {
                    group.clear();
                } else {
                    group.push(ch);
                }
            }
            ')' => {
                if depth == 0 {
                    out.push(ch);
                } else {
                    depth -= 1;
                    if depth == 0 {
                        let inner = group.trim().to_lowercase();
                        let is_noise = PAREN_KEYWORDS.iter().any(|k| inner.contains(k))
                            || is_pronunciation(&group)
                            || is_film_year(&inner);
                        if !is_noise {
                            out.push('(');
                            out.push_str(&group);
                            out.push(')');
                        }
                    } else {
                        group.push(ch);
                    }
                }
            }
            _ => {
                if depth == 0 {
                    out.push(ch);
                } else {
                    group.push(ch);
                }
            }
        }
    }

    // Unbalanced opening paren: keep the buffered tail instead of eating it.
    if depth > 0 {
        out.push('(');
        out.push_str(&group);
    }

    out
}

/// `/rks/` — IPA pronunciation guides.
fn is_pronunciation(inner: &str) -> bool {
    let t = inner.trim();
    t.len() >= 2 && t.starts_with('/') && t.ends_with('/')
}

/// `(Dune, 2021)` — modern film-year references (Dune chronology uses AG
/// years in the 10xxx range, so 19xx/20xx are safe to treat as ours).
fn is_film_year(inner: &str) -> bool {
    let t = inner.trim();
    if t.chars().count() > 24 || !t.contains(char::is_numeric) {
        return false;
    }
    let mut nums = t.split_whitespace().filter(|w| w.chars().all(|c| c.is_ascii_digit()));
    nums.any(|w| {
        matches!(w.len(), 4) && matches!(w.get(..2), Some("19" | "20"))
    })
}

/// Splits into sentences (keeping their terminator) and drops ones that are
/// only film / adaptation trivia.
fn drop_trivial_sentences(line: &str) -> String {
    let mut rebuilt = String::new();
    for sentence in split_sentences(line) {
        let lower = sentence.to_lowercase();
        let words = lower.split_whitespace().count();
        let trivial = lower.contains("fanart")
            || lower.contains("fan art")
            || lower.contains("portrayed by")
            || lower.contains("played by")
            || lower.contains("miniseries")
            || ((lower.contains("film") || lower.contains("movie")) && words <= 12);
        if !trivial {
            rebuilt.push_str(sentence);
        }
    }
    rebuilt
}

fn split_sentences(s: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let bytes = s.as_bytes();
    let mut start = 0usize;

    for i in 0..bytes.len() {
        if matches!(bytes[i], b'.' | b'!' | b'?')
            && (i + 1 == bytes.len() || bytes[i + 1] == b' ')
        {
            sentences.push(&s[start..=i]);
            start = i + 1;
        }
    }
    if start < bytes.len() {
        sentences.push(&s[start..]);
    }
    sentences
}

fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Structured single-line path
// ---------------------------------------------------------------------------

/// Infobox labels worth keeping, mapped to their display name.
const PROFILE_LABELS: &[(&str, &str)] = &[
    ("Aliases", "Aliases"),
    ("Titles", "Titles"),
    ("Great House", "Great House"),
    ("Status", "Status"),
    ("Symbol", "Symbol"),
    ("Army", "Army"),
    ("Uniform", "Uniform"),
    ("Colors", "Colors"),
    ("Homeworld", "Homeworld"),
    ("Ruling Title", "Ruling Title"),
    ("Loyalty", "Loyalty"),
    ("Family members", "Family"),
    ("Romances", "Romances"),
    ("Affiliation", "Affiliation"),
    ("Born", "Born"),
    ("Place of birth", "Place of birth"),
    ("Died", "Died"),
    ("Place of death", "Place of death"),
    ("Eyes", "Eyes"),
    ("Hair", "Hair"),
    ("Height", "Height"),
];

/// Labels whose content is dropped entirely: wiki navigation tabs and the
/// Behind-the-scenes / actor block.
const NOISE_LABELS: &[&str] = &[
    "Film series",
    "Miniseries",
    "Encyclopedia",
    "Novels",
    "Behind the Scenes",
    "First appearance",
    "Portrayed by",
    // Section group headers (always empty values).
    "Political Information",
    "Affiliation and Relationships",
    "Biographical Information",
    "Physical Characteristics",
];

/// Leading image-caption artifacts glued before the first word.
const LEADING_NOISE: &[&str] = &["Artist's portrayal", "Artist portrayal"];

/// Entry point: cleans an entity description regardless of the shape the
/// API returned. `name_hint` is the entity's display name (used to locate
/// where the biography starts after the infobox).
pub fn clean_entity_text(name_hint: &str, raw: &str) -> String {
    if raw.contains('\n') {
        clean_wiki_text(raw)
    } else {
        parse_single_line(name_hint, raw)
    }
}

#[derive(Debug, Clone, Copy)]
enum Mark {
    Field(&'static str),
    Noise,
}

/// Parses the glued single-line format into a structured entry:
///
/// ```text
/// ## Profile
/// Aliases: ...
/// Born: 10176 AG · Caladan
///
/// <intro + biography in paragraphs>
/// ```
fn parse_single_line(name_hint: &str, text: &str) -> String {
    // The API occasionally concatenates the whole description twice; keep
    // a single copy before doing anything else.
    let text = dedupe_repeated(text);
    let marks = find_marks(text);

    // No real infobox here (fewer than two labeled fields means we matched
    // a phrase like "...whichever Great House had possession..." inside
    // plain prose): render as biography only, dropping any noise tail.
    let field_count = marks
        .iter()
        .filter(|&(_, m)| matches!(m, Mark::Field(_)))
        .count();
    if field_count < 2 {
        let body = match marks.iter().find(|&(_, m)| matches!(m, Mark::Noise)) {
            Some(&(pos, _)) => &text[..pos],
            None => text,
        };
        return render_bio_only("", body);
    }

    // No structure at all: treat everything as prose (e.g. glossary terms).
    if marks.is_empty() {
        return render_bio_only("", text);
    }

    let first_pos = marks[0].0;
    let mut intro = text[..first_pos].trim().to_string();

    // The scraper repeats the entity caption right before the noise block
    // ("...concubine of Emperor Paul Atreides.Chani Kynes Film series..."):
    // drop that trailing title-like fragment.
    let hint = name_hint.trim().to_lowercase();
    intro = strip_trailing_caption(intro.trim(), &hint).to_string();
    // A bare caption identical to the entity name carries no information.
    let intro = if intro.eq_ignore_ascii_case(name_hint.trim()) {
        ""
    } else {
        intro.as_str()
    };

    let mut fields: Vec<(&'static str, String)> = Vec::new();
    let mut bio_chunks: Vec<String> = Vec::new();
    let count = marks.len();

    for (idx, &(pos, mark)) in marks.iter().enumerate() {
        let value_start = pos + label_len_at(text, pos);
        let value_end = marks.get(idx + 1).map_or(text.len(), |&(p, _)| p);
        let chunk = text[value_start..value_end].trim();
        let is_last = idx + 1 == count;

        match mark {
            Mark::Noise => {
                if is_last {
                    // Actors and trivia after the final noise label; the
                    // biography resumes at the entity's own name. A bare
                    // leftover caption without a sentence end is not a bio.
                    let (_, bio_text) = split_trailing(name_hint, chunk);
                    let trimmed = bio_text.trim();
                    if trimmed.contains(". ") || trimmed.ends_with('.') {
                        bio_chunks.push(trimmed.to_string());
                    }
                }
            }
            Mark::Field(display) => {
                if chunk.is_empty() {
                    continue;
                }
                // A field that swallowed the biography (its label repeated
                // deep in the prose): keep only the head as the value.
                if let Some(cut) = find_bio_swallow(&hint, chunk) {
                    let head = chunk[..cut].trim();
                    if !head.is_empty() {
                        push_field(&mut fields, display, fix_value_spacing(head));
                    }
                    let bio_head = strip_trailing_caption(chunk[cut..].trim(), &hint);
                    if !bio_head.is_empty() {
                        bio_chunks.push(bio_head.to_string());
                    }
                } else if is_last {
                    let (residual, bio_text) = split_trailing(name_hint, chunk);
                    if let Some(residual) = residual {
                        let cleaned = fix_value_spacing(residual.trim());
                        if !cleaned.is_empty() && !cleaned.eq_ignore_ascii_case(display) {
                            push_field(&mut fields, display, cleaned);
                        }
                    }
                    bio_chunks.push(bio_text.trim().to_string());
                } else {
                    push_field(&mut fields, display, fix_value_spacing(chunk));
                }
            }
        }
    }

    let mut out: Vec<String> = Vec::new();

    if !fields.is_empty() {
        out.push("## Profile".to_string());
        for (label, value) in &fields {
            out.push(format!("{}: {}", label, value));
        }
        out.push(String::new());
    }

    out.extend(render_bio(intro, &bio_chunks.join("\n\n")));
    out.join("\n")
}

/// Returns the byte index where a field value stops being infobox data and
/// has swallowed running prose: the first mention of the entity whose
/// suffix reads like narrative text rather than a list item.
fn find_bio_swallow(hint_lower: &str, value: &str) -> Option<usize> {
    if hint_lower.is_empty() {
        return None;
    }
    let lower = value.to_lowercase();
    let hlen = hint_lower.len();
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find(hint_lower) {
        let idx = search_from + rel;
        search_from = idx + hlen;
        if !value.is_char_boundary(idx) {
            continue;
        }
        // Reject matches glued inside a larger word on both sides.
        let prev_alpha = value[..idx].chars().next_back().is_some_and(char::is_alphabetic);
        let next_alpha = value[idx + hlen..]
            .chars()
            .next()
            .is_some_and(char::is_alphabetic);
        if prev_alpha && next_alpha {
            continue;
        }
        let suffix = &value[idx..];
        if suffix.contains(". ") || suffix.len() > 250 {
            return Some(idx);
        }
    }
    None
}

/// Drops a bare trailing caption repeat ("...Ghanima Atreides.Chani Kynes"
/// → "...Ghanima Atreides."). The caption may be the full wiki title, so
/// match a short punctuation-less title-like suffix starting with the
/// entity's own name.
fn strip_trailing_caption<'a>(text: &'a str, hint_lower: &str) -> &'a str {
    let t = text.trim_end();
    if hint_lower.is_empty() {
        return t;
    }
    let lower = t.to_lowercase();
    if let Some(pos) = lower.rfind(hint_lower)
        && pos > 0 && t.is_char_boundary(pos) {
            let prev = t.as_bytes()[pos - 1];
            let tail = &t[pos..];
            if matches!(prev, b' ' | b'.')
                && tail.split_whitespace().count() <= 4
                && !tail.contains('.')
            {
                return t[..pos].trim_end();
            }
        }
    t
}

/// Detects an exact repetition of the payload head later in the text
/// (the API sometimes serves the description concatenated twice) and
/// truncates at the start of the second copy.
fn dedupe_repeated(text: &str) -> &str {
    const HEAD_CHARS: usize = 96;
    let head_end = text
        .char_indices()
        .nth(HEAD_CHARS)
        .map_or(text.len(), |(i, _)| i);
    if head_end == text.len() {
        return text;
    }
    let head = &text[..head_end];
    if let Some(offset) = text[head_end..].find(head) {
        let pos = head_end + offset;
        if text.is_char_boundary(pos) && text.len() - pos >= text.len() / 3 {
            return &text[..pos];
        }
    }
    text
}

/// Renders the case where there are no profile fields at all.
fn render_bio_only(_intro: &str, text: &str) -> String {
    render_bio("", text).join("\n")
}

fn find_marks(text: &str) -> Vec<(usize, Mark)> {
    let bytes = text.as_bytes();
    let mut marks = Vec::new();
    let mut i = 0usize;

    while i < text.len() {
        // A label never starts in the middle of a word.
        if i > 0 && bytes[i - 1].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        if let Some((len, kind)) = match_mark(&text[i..]) {
            let end = i + len;
            if end == text.len() || bytes[end] == b' ' {
                marks.push((i, kind));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    marks
}

fn match_mark(s: &str) -> Option<(usize, Mark)> {
    let mut best: Option<(usize, Mark)> = None;
    for (raw, display) in PROFILE_LABELS {
        if s.starts_with(raw) && best.as_ref().is_none_or(|(l, _)| *l < raw.len()) {
            best = Some((raw.len(), Mark::Field(display)));
        }
    }
    for raw in NOISE_LABELS {
        if s.starts_with(raw) && best.as_ref().is_none_or(|(l, _)| *l < raw.len()) {
            best = Some((raw.len(), Mark::Noise));
        }
    }
    best
}

fn label_len_at(text: &str, pos: usize) -> usize {
    match_mark(&text[pos..]).map_or(0, |(l, _)| l)
}

/// Splits the chunk following the final mark into `(residual value, bio)`.
/// The bio begins at the first mention of the entity's own name; whatever
/// precedes it belonged to the last field (e.g. actors after "Portrayed
/// by", or "God-Emperor" after "Ruling Title").
fn split_trailing<'a>(name_hint: &str, trailing: &'a str) -> (Option<&'a str>, &'a str) {
    let lower_hint = name_hint.trim().to_lowercase();
    if lower_hint.is_empty() {
        return (None, trailing);
    }
    let lower = trailing.to_lowercase();
    if let Some(idx) = lower.find(&lower_hint) {
        if idx > 0 && trailing.is_char_boundary(idx) {
            let (before, after) = trailing.split_at(idx);
            return (
                Some(before.trim_end_matches([' ', ',', ';'])),
                after,
            );
        }
        if idx == 0 {
            return (None, trailing);
        }
    }
    // Fallback: no self-mention found. If the previous mark was noise we
    // cannot trust any of it; keep everything as bio when it looks like
    // prose, otherwise attach it to the last field via `None`+full text.
    (None, trailing)
}

/// Inserts a field, merging Born/Place-of-birth and Died/Place-of-death
/// pairs onto one line.
fn push_field(fields: &mut Vec<(&'static str, String)>, label: &'static str, value: String) {
    if value.is_empty() {
        return;
    }
    const PAIRS: &[(&str, &str)] = &[("Place of birth", "Born"), ("Place of death", "Died")];
    if let Some((_, anchor)) = PAIRS.iter().find(|(place, _)| *place == label)
        && let Some(entry) = fields.iter_mut().find(|(l, _)| l == anchor) {
            entry.1.push_str(&format!(" · {}", value));
            return;
        }
    fields.push((label, value));
}

/// Repairs missing separators inside infobox values: glued CamelCase words
/// become spaced, parenthesized item lists get `·` separators, and letters
/// glued to parens are separated.
fn fix_value_spacing(value: &str) -> String {
    let spaced = insert_glue_spaces(value);
    collapse_spaces(spaced.trim())
}

/// Repairs prose: drops noise parens, glue spacing, sentence boundaries.
fn fix_prose_spacing(text: &str) -> String {
    let no_parens = strip_noisy_parentheticals(text);
    let spaced = insert_glue_spaces(&no_parens);
    let repaired = repair_sentence_boundaries(&spaced);
    repaired.replace(" ,", ",")
}

/// Single pass inserting spaces where the scraper glued tokens together.
fn insert_glue_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut prev: Option<char> = None;

    for ch in text.chars() {
        if let Some(p) = prev {
            let boundary = p.is_lowercase() && ch.is_uppercase() && !(p == 'c' && ch == 'K');
            if boundary || (p.is_ascii_alphanumeric() && ch == '(') {
                out.push(' ');
            } else if p == ')' && (ch.is_uppercase() || ch == '(') {
                // Item lists like "(wife)Tanidia Nerus (...)".
                out.push_str(" · ");
            }
        }
        out.push(ch);
        prev = Some(ch);
    }
    out
}

/// `"...Atreides II.House Atreides..."` → `"...II. House Atreides..."`.
fn repair_sentence_boundaries(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 8);

    for (i, &ch) in chars.iter().enumerate() {
        out.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let next = chars.get(i + 1);
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if let Some(n) = next
                && n.is_uppercase() && *n != ' ' && prev != Some(' ') {
                    // Avoid breaking abbreviations like "St.Paul"? Rare here;
                    // only split when the terminator follows a letter/digit.
                    if prev.is_some_and(|p| p.is_alphanumeric()) {
                        out.push(' ');
                    }
                }
        }
    }
    out
}

/// Renders intro + bio: cleans them up and groups sentences into
/// paragraphs of at most [`SENTENCES_PER_PARA`] sentences.
fn render_bio(intro: &str, bio: &str) -> Vec<String> {
    const SENTENCES_PER_PARA: usize = 5;

    let mut paragraphs: Vec<String> = Vec::new();

    let intro_trim = strip_leading_noise(intro.trim());
    if !intro_trim.is_empty() {
        paragraphs.push(fix_prose_spacing(&intro_trim));
    }

    let cleaned = fix_prose_spacing(&strip_leading_noise(bio.trim()));
    if !cleaned.is_empty() {
        let sentences = split_sentences(&cleaned);
        let mut current: Vec<&str> = Vec::new();
        for sentence in sentences {
            current.push(sentence);
            if current.len() >= SENTENCES_PER_PARA {
                paragraphs.push(current.concat());
                current.clear();
            }
        }
        if !current.is_empty() {
            paragraphs.push(current.concat());
        }
    }

    let joined: Vec<String> = paragraphs
        .into_iter()
        .filter(|p| !collapse_spaces(p).is_empty())
        .map(|p| collapse_spaces(&p))
        .collect();

    if joined.is_empty() {
        Vec::new()
    } else {
        vec![joined.join("\n\n")]
    }
}

fn strip_leading_noise(text: &str) -> String {
    let mut t = text.trim();
    for prefix in LEADING_NOISE {
        if t.len() > prefix.len() && t[..prefix.len()].eq_ignore_ascii_case(prefix) {
            t = t[prefix.len()..]
                .trim_start_matches([' ', ',', '-'])
                .trim_start();
            break;
        }
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_fanart_parenthetical() {
        let cleaned = clean_wiki_text(
            "Paul Muad'Dib Atreides (fanart by Paul Hanley) is the son of Duke Leto.",
        );
        assert_eq!(
            cleaned,
            "Paul Muad'Dib Atreides is the son of Duke Leto."
        );
    }

    #[test]
    fn keeps_normal_parentheticals() {
        let cleaned = clean_wiki_text("He ruled as Duke (10191 AG) until his death.");
        assert_eq!(cleaned, "He ruled as Duke (10191 AG) until his death.");
    }

    #[test]
    fn drops_trivial_film_fragments() {
        let cleaned = clean_wiki_text(
            "Paul Atreides led the Fremen. Miniseries 1984 film. He became Emperor.",
        );
        assert_eq!(cleaned, "Paul Atreides led the Fremen. He became Emperor.");
    }

    #[test]
    fn drops_portrayed_by_references() {
        let cleaned =
            clean_wiki_text("Paul was portrayed by Timothe Chalamet in the 2021 adaptation.");
        // The whole line was trivia, so nothing remains.
        assert_eq!(cleaned, "");
    }

    #[test]
    fn separates_and_normalizes_sections() {
        let raw = "Titles\nDuke of House Atreides\nEmperor\nBiography Paul was born on \
                   Caladan as the heir of House Atreides.";
        let cleaned = clean_wiki_text(raw);
        assert!(cleaned.contains("## Titles"));
        assert!(cleaned.contains("## Biography"));
        assert!(cleaned.contains("\n\n"));
        let titles_idx = cleaned.find("## Titles").unwrap();
        let bio_idx = cleaned.find("## Biography").unwrap();
        assert!(titles_idx < bio_idx);
    }

    #[test]
    fn splits_glued_headers_from_content() {
        let cleaned = clean_wiki_text("Biography Paul was born on Caladan as the heir.");
        assert!(cleaned.contains("## Biography"));
        assert!(cleaned.contains("Paul was born on Caladan as the heir."));
    }

    #[test]
    fn does_not_split_sentences_that_merely_start_like_headers() {
        let cleaned = clean_wiki_text("Titles are earned through service in the Imperium.");
        assert!(!cleaned.contains("## Titles"));
        assert_eq!(cleaned, "Titles are earned through service in the Imperium.");
    }

    #[test]
    fn drops_glued_behind_the_scenes_header() {
        let cleaned =
            clean_wiki_text("Behind the Scenes Several actors played the character on screen.");
        assert_eq!(cleaned, "");
    }

    #[test]
    fn keeps_long_biographical_sentences_mentioning_films() {
        let raw = "The 1984 film adaptation compressed the novel considerably, yet the \
                   book remains the canonical version of Paul's story for most readers.";
        let cleaned = clean_wiki_text(raw);
        assert_eq!(cleaned, raw);
    }

    // -- structured single-line path -------------------------------------

    const PAUL: &str = "Paul Atreides Film series Miniseries 1984 film Novels Aliases \
Muad'Dib, Usul, The Preacher Political Information Titles Duke, Padishah Emperor Great House \
House Atreides Affiliation and Relationships Loyalty House AtreidesFremenSietch Tabr Family \
members Irulan Corrino (wife)Tanidia Nerus (maternal grandmother)Jessica Atreides (mother) \
Romances Irulan Corrino (wife)Chani Kynes (concubine) Biographical Information Born 10176 AG \
Place of birth Caladan Died 10219 AG Place of death Arrakeen, Arrakis Physical Characteristics \
Eyes Blue within blue(formerly green) Hair Black Height Short Behind the Scenes First \
appearance Portrayed by Dune(1965) Timothe Chalamet (film series)Kyle MacLachlan (1984 film)\
Paul Atreides (10176 AG - 10219 AG), predominantly known as Muad'Dib, was the last Duke of \
the noble House Atreides.He rose to power among the Fremen.Their victory led him to \
overthrow the Emperor Shaddam IV.He formed the Atreides Empire.";

    #[test]
    fn parses_profile_block_from_real_payload() {
        let cleaned = clean_entity_text("Paul Atreides", PAUL);

        assert!(cleaned.starts_with("## Profile"));
        assert!(cleaned.contains("Aliases: Muad'Dib, Usul, The Preacher"));
        assert!(cleaned.contains("Titles: Duke, Padishah Emperor"));
        assert!(cleaned.contains("Great House: House Atreides"));
        // Glued enum values get separated.
        assert!(cleaned.contains("Loyalty: House Atreides Fremen Sietch Tabr"));
        // Parenthesized lists become · separated items.
        assert!(cleaned.contains("Irulan Corrino (wife) · Tanidia Nerus (maternal grandmother)"));
        // Born/Place of birth merged on one line.
        assert!(cleaned.contains("Born: 10176 AG · Caladan"));
        assert!(cleaned.contains("Died: 10219 AG · Arrakeen, Arrakis"));
        // Glued parenthesis inside a value gets a space.
        assert!(cleaned.contains("Blue within blue (formerly green)"));
        assert!(cleaned.contains("Height: Short"));
    }

    #[test]
    fn strips_actors_and_behind_the_scenes() {
        let cleaned = clean_entity_text("Paul Atreides", PAUL);
        assert!(!cleaned.contains("Chalamet"));
        assert!(!cleaned.contains("MacLachlan"));
        assert!(!cleaned.contains("Portrayed by"));
        assert!(!cleaned.contains("Behind the Scenes"));
        assert!(!cleaned.contains("Film series"));
        assert!(!cleaned.contains("(1965)"));
    }

    #[test]
    fn bio_starts_at_self_mention_and_repairs_boundaries() {
        let cleaned = clean_entity_text("Paul Atreides", PAUL);
        assert!(cleaned.contains(
            "Paul Atreides (10176 AG - 10219 AG), predominantly known as Muad'Dib"
        ));
        // "...House Atreides.He rose..." → sentence space repaired.
        assert!(cleaned.contains("House Atreides. He rose to power among the Fremen."));
    }

    #[test]
    fn house_bio_split_without_behind_the_scenes_marker() {
        let raw = "House Atreides Status House Major Homeworld Caladan Ruling Title Duke, \
Emperor, God-EmperorHouse Atreides was one of the Houses Major within the Galactic Padishah \
Empire.They were ruled by the patriarch of the Atreides family.";
        let cleaned = clean_entity_text("House Atreides", raw);
        assert!(cleaned.contains("Status: House Major"));
        assert!(cleaned.contains("Homeworld: Caladan"));
        assert!(cleaned.contains("Ruling Title: Duke, Emperor, God-Emperor"));
        assert!(cleaned.contains(
            "House Atreides was one of the Houses Major within the Galactic Padishah Empire."
        ));
    }

    #[test]
    fn keeps_intro_paragraph_before_infobox() {
        let raw = "Chani Kynes [d. 10210 AG], also known by her intimate name Sihaya, was \
the Fremen bound concubine of Emperor Paul Atreides.Chani Kynes Encyclopedia Aliases Sihaya \
Romances Paul Atreides Behind the Scenes First appearance Portrayed by Dune(1965) Zendaya \
(film series)Sean Young (1984 film)Chani was described as skinny with an elfin face.";
        let cleaned = clean_entity_text("Chani", raw);
        assert!(cleaned.contains("also known by her intimate name Sihaya"));
        assert!(cleaned.contains("Aliases: Sihaya"));
        assert!(cleaned.contains("Romances: Paul Atreides"));
        assert!(!cleaned.contains("Zendaya"));
        assert!(!cleaned.contains("Sean Young"));
        assert!(cleaned.contains("Chani was described as skinny with an elfin face."));
    }

    #[test]
    fn pure_prose_entry_gets_no_profile_block() {
        let raw = "Artist's portrayalArrakis (/rks/), also known as 'Dune', is a harsh \
desert planet located on the far edge of the Old Imperium.It later became the Imperium's \
center under Muad'Dib's empire.It was the original source of the Spice Melange.";
        let cleaned = clean_entity_text("Arrakis", raw);
        assert!(!cleaned.contains("## Profile"));
        assert!(!cleaned.contains("Artist's portrayal"));
        assert!(!cleaned.contains("/rks/"));
        assert!(cleaned.starts_with("Arrakis, also known as 'Dune'"));
        assert!(cleaned.contains("Imperium. It later became"));
    }

    #[test]
    fn duplicated_payload_is_reduced_to_one_copy() {
        let base = "House Atreides Status House Major Homeworld Caladan Portrayed by \
Kyle MacLachlan House Atreides was an old and wealthy House Major. They ruled Caladan.";
        let cleaned = clean_entity_text("house atreides", &base.repeat(2));
        assert_eq!(cleaned.matches("## Profile").count(), 1);
        assert_eq!(cleaned.matches("Status:").count(), 1);
        assert_eq!(cleaned.matches("was an old and wealthy House Major.").count(), 1);
    }

    #[test]
    fn field_that_swallowed_bio_is_split_at_entity_name() {
        let raw = "House Atreides Status House Major Ruling Title Duke, Emperor, \
God-EmperorHouse Atreides was one of the Houses Major of the Imperium. They ruled Caladan.";
        let cleaned = clean_entity_text("house atreides", raw);
        assert!(cleaned.contains("Ruling Title: Duke, Emperor, God-Emperor\n"));
        assert!(!cleaned.contains(
            "Ruling Title: Duke, Emperor, God-Emperor House Atreides was"
        ));
        assert_eq!(
            cleaned.matches("was one of the Houses Major of the Imperium.").count(),
            1
        );
    }

    #[test]
    fn glossary_drops_film_year_reference() {
        let raw = "A handful of the spice-rich sand from a spice field (Dune, 2021)The \
Spice Melange, commonly referred to simply as 'the spice', was a naturally produced \
awareness spectrum narcotic.";
        let cleaned = clean_entity_text("spice melange", raw);
        assert!(!cleaned.contains("(Dune, 2021)"));
        assert!(cleaned.contains(
            "The Spice Melange, commonly referred to simply as 'the spice'"
        ));
    }

    #[test]
    fn long_bio_is_chunked_into_paragraphs() {
        let raw = "Entry Bio One.Two.Three.Four.Five.Six.Seven.";
        let cleaned = clean_entity_text("Entry", raw);
        assert!(cleaned.contains("\n\n"), "expected a paragraph break");
    }

    #[test]
    fn preserves_ag_dates_when_splitting_values() {
        // Lowercase→uppercase splitting must not corrupt normal values.
        assert_eq!(fix_value_spacing("Blue within blue(formerly green)"), "Blue within blue (formerly green)");
        assert_eq!(fix_value_spacing("Caladan"), "Caladan");
        assert_eq!(insert_glue_spaces("McKie"), "McKie");
    }
}
