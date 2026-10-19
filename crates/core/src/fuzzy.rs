//! Fuzzy path matching: a port of `telescope-fzf-native` (`src/fzf.c`), the fzf matcher in the user's nvim picker.
//!
//! It is fzf's `FuzzyMatchV2` (a Smith-Waterman variant with affine gaps) with the scoring of that C port,
//! which is older than fzf's current `path` scheme: `/` is an ordinary non-word character, and every boundary is worth 8.
//! The constants, the character classes, and the recurrences keep the C names and order, so the two can be compared.
//! `scripts/fzf-golden.py` checks the port against the compiled C library.

use std::cmp::Reverse;

const SCORE_MATCH: i32 = 16;
const SCORE_GAP_START: i32 = -3;
const SCORE_GAP_EXTENSION: i32 = -1;
const BONUS_BOUNDARY: i32 = SCORE_MATCH / 2;
const BONUS_NON_WORD: i32 = SCORE_MATCH / 2;
const BONUS_CAMEL123: i32 = BONUS_BOUNDARY + SCORE_GAP_EXTENSION;
const BONUS_CONSECUTIVE: i32 = -(SCORE_GAP_START + SCORE_GAP_EXTENSION);
const BONUS_FIRST_CHAR_MULTIPLIER: i32 = 2;
/// The class before the first character: a path starts as if after a non-word character.
const INITIAL_CLASS: Class = Class::NonWord;

/// The C port's character classes. It reads bytes, so it has no classes for non-ASCII text;
/// this port reads chars and gives non-ASCII letters and digits their Unicode classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    NonWord,
    Lower,
    Upper,
    Letter,
    Number,
}

fn class_of(c: char) -> Class {
    match c {
        'a'..='z' => Class::Lower,
        'A'..='Z' => Class::Upper,
        '0'..='9' => Class::Number,
        _ if c.is_ascii() => Class::NonWord,
        _ if c.is_lowercase() => Class::Lower,
        _ if c.is_uppercase() => Class::Upper,
        _ if c.is_numeric() => Class::Number,
        _ if c.is_alphabetic() => Class::Letter,
        _ => Class::NonWord,
    }
}

/// `bonus_for`: the bonus for matching a character of `class` that follows a character of `prev`.
fn bonus_for(prev: Class, class: Class) -> i32 {
    if prev == Class::NonWord && class != Class::NonWord {
        return BONUS_BOUNDARY;
    }
    let camel = prev == Class::Lower && class == Class::Upper;
    if camel || prev != Class::Number && class == Class::Number {
        return BONUS_CAMEL123;
    }
    if class == Class::NonWord {
        return BONUS_NON_WORD;
    }
    0
}

fn lower(c: char) -> char {
    if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// A path prepared for matching: its chars, lowercased chars, and the bonus of each position.
struct Text {
    chars: Vec<char>,
    lower: Vec<char>,
    bonus: Vec<i32>,
}

impl Text {
    fn new(s: &str) -> Text {
        let chars: Vec<char> = s.chars().collect();
        let lower = chars.iter().map(|&c| lower(c)).collect();
        let mut prev = INITIAL_CLASS;
        let bonus = chars
            .iter()
            .map(|&c| {
                let class = class_of(c);
                let b = bonus_for(prev, class);
                prev = class;
                b
            })
            .collect();
        Text {
            chars,
            lower,
            bonus,
        }
    }

    fn chars(&self, case_sensitive: bool) -> &[char] {
        if case_sensitive {
            &self.chars
        } else {
            &self.lower
        }
    }
}

/// `fzf_fuzzy_match_v2`: the best-scoring alignment of `pat` as a subsequence of the text, and its positions.
fn fuzzy(text: &Text, pat: &[char], case_sensitive: bool) -> Option<(i32, Vec<usize>)> {
    let t = text.chars(case_sensitive);
    let (n, m) = (t.len(), pat.len());
    if m == 0 {
        return Some((0, Vec::new()));
    }
    // The earliest position of each pattern char (`f`); no match unless the pattern is a subsequence.
    let mut first = Vec::with_capacity(m);
    let mut j = 0;
    for &p in pat {
        while j < n && t[j] != p {
            j += 1;
        }
        if j == n {
            return None;
        }
        first.push(j);
        j += 1;
    }
    // The last position of the last pattern char (`last_idx`): no alignment ends after it.
    let last = (first[m - 1]..n).rev().find(|&j| t[j] == pat[m - 1])?;
    let b = &text.bonus;

    if m == 1 {
        let (mut best, mut pos) = (0, first[0]);
        for j in first[0]..=last {
            if t[j] == pat[0] {
                let score = SCORE_MATCH + b[j] * BONUS_FIRST_CHAR_MULTIPLIER;
                if score > best {
                    (best, pos) = (score, j);
                    if b[j] == BONUS_BOUNDARY {
                        break;
                    }
                }
            }
        }
        return Some((best, vec![pos]));
    }

    // Score matrix `h` and consecutive-run matrix `c`, one row per pattern char, columns `f0..=last`.
    let f0 = first[0];
    let width = last - f0 + 1;
    let mut h = vec![0i32; m * width];
    let mut c = vec![0i32; m * width];
    let (mut prev_h, mut in_gap) = (0, false);
    for j in f0..=last {
        let k = j - f0;
        if t[j] == pat[0] {
            h[k] = SCORE_MATCH + b[j] * BONUS_FIRST_CHAR_MULTIPLIER;
            c[k] = 1;
            in_gap = false;
        } else {
            let gap = if in_gap {
                SCORE_GAP_EXTENSION
            } else {
                SCORE_GAP_START
            };
            h[k] = (prev_h + gap).max(0);
            c[k] = 0;
            in_gap = true;
        }
        prev_h = h[k];
    }
    let (mut max_score, mut max_pos) = (0, last);
    for i in 1..m {
        let row = i * width;
        let mut in_gap = false;
        for j in first[i]..=last {
            let k = j - f0;
            let left = if j > first[i] { h[row + k - 1] } else { 0 };
            let gap = if in_gap {
                SCORE_GAP_EXTENSION
            } else {
                SCORE_GAP_START
            };
            let s2 = left + gap;
            let (mut s1, mut consecutive) = (0, 0);
            if t[j] == pat[i] {
                s1 = h[row - width + k - 1] + SCORE_MATCH;
                let mut bonus = b[j];
                consecutive = c[row - width + k - 1] + 1;
                // A boundary starts a new consecutive chunk; inside a chunk, the chunk's first bonus carries on.
                if bonus == BONUS_BOUNDARY {
                    consecutive = 1;
                } else if consecutive > 1 {
                    bonus = bonus.max(BONUS_CONSECUTIVE.max(b[j + 1 - consecutive as usize]));
                }
                if s1 + bonus < s2 {
                    s1 += b[j];
                    consecutive = 0;
                } else {
                    s1 += bonus;
                }
            }
            c[row + k] = consecutive;
            in_gap = s1 < s2;
            let score = s1.max(s2).max(0);
            if i == m - 1 && score > max_score {
                (max_score, max_pos) = (score, j);
            }
            h[row + k] = score;
        }
    }

    // Backtrace from the best end to the positions.
    let mut pos = Vec::with_capacity(m);
    let (mut i, mut j) = (m - 1, max_pos);
    let mut prefer_match = true;
    loop {
        let (row, k) = (i * width, j - f0);
        let s = h[row + k];
        let s1 = if i > 0 && j >= first[i] {
            h[row - width + k - 1]
        } else {
            0
        };
        let s2 = if j > first[i] { h[row + k - 1] } else { 0 };
        if s > s1 && (s > s2 || s == s2 && prefer_match) {
            pos.push(j);
            if i == 0 {
                break;
            }
            i -= 1;
        }
        let below = row + width + k + 1;
        prefer_match = c[row + k] > 1 || below < c.len() && c[below] > 0;
        if j == 0 {
            break;
        }
        j -= 1;
    }
    pos.reverse();
    Some((max_score, pos))
}

/// `calculate_score`: the score of a pattern matched at `start..start + len`, which the caller has checked.
fn span_score(text: &Text, start: usize, len: usize) -> i32 {
    let b = &text.bonus;
    let (mut score, mut first_bonus) = (0, 0);
    for (q, j) in (start..start + len).enumerate() {
        score += SCORE_MATCH;
        let mut bonus = b[j];
        if q == 0 {
            first_bonus = bonus;
            score += bonus * BONUS_FIRST_CHAR_MULTIPLIER;
        } else {
            // A boundary starts a new consecutive chunk.
            if bonus == BONUS_BOUNDARY {
                first_bonus = bonus;
            }
            bonus = bonus.max(first_bonus).max(BONUS_CONSECUTIVE);
            score += bonus;
        }
    }
    score
}

fn occurs_at(t: &[char], pat: &[char], start: usize) -> bool {
    start + pat.len() <= t.len() && t[start..start + pat.len()] == *pat
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Fuzzy,
    Exact,
    Prefix,
    Suffix,
    Equal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Term {
    kind: Kind,
    inverse: bool,
    case_sensitive: bool,
    /// Lowercased unless the term is case-sensitive.
    text: Vec<char>,
}

impl Term {
    /// `fzf_parse_pattern` for one term, in the C order: `!` inverts (exact), `$` anchors the end,
    /// then `'` makes it exact (fuzzy when inverted) or `^` anchors the start. Smart case looks at the whole token.
    fn parse(token: &str) -> Option<Term> {
        let case_sensitive = token.chars().any(|c| c.is_ascii_uppercase());
        let mut s = token;
        let inverse = s.starts_with('!');
        let mut kind = Kind::Fuzzy;
        if inverse {
            kind = Kind::Exact;
            s = &s[1..];
        }
        if s != "$" && s.ends_with('$') {
            kind = Kind::Suffix;
            s = &s[..s.len() - 1];
        }
        if let Some(rest) = s.strip_prefix('\'') {
            kind = if inverse { Kind::Fuzzy } else { Kind::Exact };
            s = rest;
        } else if let Some(rest) = s.strip_prefix('^') {
            kind = if kind == Kind::Suffix {
                Kind::Equal
            } else {
                Kind::Prefix
            };
            s = rest;
        }
        if s.is_empty() {
            return None;
        }
        let text = s
            .chars()
            .map(|c| if case_sensitive { c } else { lower(c) })
            .collect();
        Some(Term {
            kind,
            inverse,
            case_sensitive,
            text,
        })
    }

    /// The term's score and positions, or `None` when it does not match.
    fn score(&self, text: &Text) -> Option<(i32, Vec<usize>)> {
        let t = text.chars(self.case_sensitive);
        let p = &self.text;
        let span = |start: usize| {
            let positions = (start..start + p.len()).collect();
            (span_score(text, start, p.len()), positions)
        };
        let found = match self.kind {
            Kind::Fuzzy => fuzzy(text, p, self.case_sensitive),
            Kind::Prefix => occurs_at(t, p, 0).then(|| span(0)),
            Kind::Suffix => {
                let start = t.len().checked_sub(p.len());
                start.filter(|&s| occurs_at(t, p, s)).map(span)
            }
            Kind::Equal => (t.len() == p.len() && occurs_at(t, p, 0)).then(|| span(0)),
            Kind::Exact => {
                // The occurrence whose first char has the highest bonus (`fzf_exact_match_naive`).
                let mut best: Option<usize> = None;
                for s in 0..=t.len().saturating_sub(p.len()) {
                    if occurs_at(t, p, s) && best.is_none_or(|b| text.bonus[s] > text.bonus[b]) {
                        best = Some(s);
                        if text.bonus[s] == BONUS_BOUNDARY {
                            break;
                        }
                    }
                }
                best.map(span)
            }
        };
        match (self.inverse, found) {
            (false, found) => found,
            (true, None) => Some((0, Vec::new())),
            (true, Some(_)) => None,
        }
    }
}

/// A parsed query: terms split on spaces, and every term must match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pattern {
    terms: Vec<Term>,
}

/// A path that matches: its score and the char indices of the matched characters, sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub score: i32,
    pub positions: Vec<usize>,
}

impl Pattern {
    /// Parses fzf's extended search syntax: `foo` fuzzy, `'foo` exact, `^foo` prefix, `foo$` suffix, `!foo` not.
    /// Terms split on spaces, and all must match. A term with an uppercase letter is case-sensitive (smart case).
    /// Not supported: `|` (OR) and `\ ` (an escaped space).
    pub fn parse(query: &str) -> Pattern {
        Pattern {
            terms: query.split(' ').filter_map(Term::parse).collect(),
        }
    }

    /// True when the query has no terms. It then matches every path with score 0.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn score(&self, path: &str) -> Option<Match> {
        self.score_candidate(&Candidate::new(path))
    }

    /// Scores a path prepared once with [`Candidate::new`], for ranking many paths per keystroke.
    pub fn score_candidate(&self, candidate: &Candidate) -> Option<Match> {
        let text = &candidate.text;
        let mut out = Match {
            score: 0,
            positions: Vec::new(),
        };
        for term in &self.terms {
            let (score, positions) = term.score(text)?;
            out.score += score;
            out.positions.extend(positions);
        }
        out.positions.sort_unstable();
        out.positions.dedup();
        Some(out)
    }
}

/// A path prepared for matching: its chars, lowercased chars, bonuses, and where its file name starts.
pub struct Candidate {
    text: Text,
    name_start: usize,
}

impl Candidate {
    pub fn new(path: &str) -> Candidate {
        let text = Text::new(path);
        let name_start = text
            .chars
            .iter()
            .rposition(|&c| c == '/')
            .map_or(0, |s| s + 1);
        Candidate { text, name_start }
    }
}

/// A ranked path: its index in the input, and its match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    pub index: usize,
    pub score: i32,
    pub positions: Vec<usize>,
}

/// The matching paths, best first: by score, then with the first match in the file name, then shorter, then in
/// input order. With an empty pattern every path matches, in input order.
pub fn rank<'a>(
    candidates: impl IntoIterator<Item = (usize, &'a Candidate)>,
    pattern: &Pattern,
) -> Vec<Ranked> {
    let mut out: Vec<(Ranked, bool, usize)> = Vec::new();
    for (index, candidate) in candidates {
        if let Some(m) = pattern.score_candidate(candidate) {
            let in_name = m
                .positions
                .first()
                .is_some_and(|&p| p >= candidate.name_start);
            let len = candidate.text.chars.len();
            let r = Ranked {
                index,
                score: m.score,
                positions: m.positions,
            };
            out.push((r, in_name, len));
        }
    }
    if !pattern.is_empty() {
        out.sort_by_key(|(r, in_name, len)| (Reverse(r.score), !in_name, *len, r.index));
    }
    out.into_iter().map(|(r, _, _)| r).collect()
}
