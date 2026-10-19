//! Text fitting and number formats from `mockups/v3-template.html:203-210`, for both front ends (unit `044`).
//! Widths count chars. Each char takes one cell, or one monospace advance in the GUI.

/// Cuts the end: `abcdefgh` at 5 is `abcd…`.
pub fn fit(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if w == 0 {
        String::new()
    } else if n <= w {
        s.to_string()
    } else if w <= 1 {
        s.chars().take(w).collect()
    } else {
        s.chars().take(w - 1).chain(['…']).collect()
    }
}

/// Cuts the start: `abcdefgh` at 5 is `…efgh`.
pub fn fit_left(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if w == 0 {
        String::new()
    } else if n <= w {
        s.to_string()
    } else {
        ['…'].into_iter().chain(s.chars().skip(n - w + 1)).collect()
    }
}

/// Cuts the middle, keeping 60% of the room for the start: `abcdefghij` at 7 is `abcd…ij`.
pub fn fit_mid(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if w == 0 {
        return String::new();
    }
    if n <= w {
        return s.to_string();
    }
    if w < 5 {
        return fit(s, w);
    }
    let a = ((w - 1) as f64 * 0.6).ceil() as usize;
    let b = w - 1 - a;
    s.chars()
        .take(a)
        .chain(['…'])
        .chain(s.chars().skip(n - b))
        .collect()
}

/// Word-wraps to lines of at most `w` chars. A word longer than `w` breaks across lines (`wrap`, v5 mockup).
pub fn wrap(s: &str, w: usize) -> Vec<String> {
    let mut out = Vec::new();
    if w == 0 {
        return out;
    }
    let mut line = String::new();
    for word in s.split_whitespace() {
        let (n, k) = (line.chars().count(), word.chars().count());
        if n > 0 && n + 1 + k > w {
            out.push(std::mem::take(&mut line));
        } else if n > 0 {
            line.push(' ');
        }
        line.push_str(word);
        while line.chars().count() > w {
            let rest: String = line.chars().skip(w).collect();
            line = line.chars().take(w).collect();
            out.push(std::mem::replace(&mut line, rest));
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// `1234567` is `1,234,567`, as `toLocaleString()` writes it in English.
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `338779` is `339k`, `12345` is `12.3k`, `999` is `999` (`kfmt`, `:210`). Halves round up, as `toFixed` does.
pub fn kilo(n: u64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 100_000 {
        let tenths = (n + 50) / 100;
        format!("{}.{}k", tenths / 10, tenths % 10)
    } else {
        format!("{}k", (n + 500) / 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits() {
        assert_eq!(fit("abcdefgh", 5), "abcd…");
        assert_eq!(fit("abc", 5), "abc");
        assert_eq!(fit("abc", 1), "a");
        assert_eq!(fit_left("abcdefgh", 5), "…efgh");
        assert_eq!(fit_mid("abcdefghij", 7), "abcd…ij");
        assert_eq!(
            fit_mid("berth_planner_test_helpers.clj", 18),
            "berth_plann…rs.clj"
        );
        assert_eq!(fit_mid("abcdefgh", 4), "abc…");
    }

    #[test]
    fn wraps() {
        assert_eq!(
            wrap("bases/api/src/quayside/", 8),
            ["bases/ap", "i/src/qu", "ayside/"]
        );
        assert_eq!(wrap("a bb ccc", 4), ["a bb", "ccc"]);
        assert_eq!(wrap("", 4), Vec::<String>::new());
    }

    #[test]
    fn numbers() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(2585), "2,585");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(kilo(999), "999");
        assert_eq!(kilo(12250), "12.3k");
        assert_eq!(kilo(12249), "12.2k");
        assert_eq!(kilo(338779), "339k");
    }
}
