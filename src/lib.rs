//! SigmaOS-apt-listbugs — Debian bug-list parser & filter.
//!
//! Inspired by Debian's `apt-listbugs`: parses the `debian-bugs` style bug
//! reports apt shows before installing — entries tagged with severity
//! (`critical`, `important`, `normal`, `minor`, `wishlist`) and optional
//! tags — into structured records for display and severity filtering.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Errors raised by this component.
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    /// The input had bug text but no parseable entries.
    #[error("no bug entries found")]
    NoEntries,
}

/// Result alias for component operations.
pub type ComponentResult<T> = Result<T, ComponentError>;

/// Bug severity, ordered most→least severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Blocks the release / data loss.
    Critical,
    /// Serious problem.
    Important,
    /// Ordinary bug.
    Normal,
    /// Minor issue.
    Minor,
    /// Nice-to-have.
    Wishlist,
    /// Unspecified — sorts lowest.
    Unknown,
}

impl Severity {
    /// Parse a severity token.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "critical" | "blocker" => Severity::Critical,
            "important" | "major" => Severity::Important,
            "normal" | "major?" => Severity::Normal,
            "minor" => Severity::Minor,
            "wishlist" | "whishlist" => Severity::Wishlist,
            _ => Severity::Unknown,
        }
    }
}

/// One bug report entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bug {
    /// Bug number (if present).
    pub number: Option<u64>,
    /// Severity.
    pub severity: Severity,
    /// Package the bug affects.
    pub package: String,
    /// Summary/title line.
    pub summary: String,
    /// Extra tags (e.g. `patch`, `security`).
    pub tags: Vec<String>,
}

/// Parsed bug list.
#[derive(Debug, Default)]
pub struct BugList {
    bugs: Vec<Bug>,
}

impl BugList {
    /// Empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse `debian-bugs`/`apt-listbugs` style text. Entries are blocks
    /// separated by blank lines; fields like `Bug:`, `Severity:`, `Package:`,
    /// `Tags:` and a title line.
    pub fn parse(input: &str) -> ComponentResult<Self> {
        let mut list = BugList::new();
        let mut cur: Option<Bug> = None;
        let mut saw_any = false;

        for raw in input.lines() {
            let line = raw.trim_end();
            if line.trim().is_empty() {
                if let Some(b) = cur.take() { list.bugs.push(b); }
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim().to_ascii_lowercase();
                let v = v.trim();
                match k.as_str() {
                    "bug" => {
                        if let Some(b) = cur.take() { list.bugs.push(b); }
                        let num = v.trim_start_matches('#').split_whitespace().next()
                            .and_then(|s| s.parse().ok());
                        cur = Some(Bug {
                            number: num, severity: Severity::Unknown,
                            package: String::new(), summary: String::new(), tags: Vec::new(),
                        });
                        saw_any = true;
                    }
                    "severity" => {
                        if let Some(b) = cur.as_mut() { b.severity = Severity::parse(v); }
                    }
                    "package" => {
                        if let Some(b) = cur.as_mut() { b.package = v.to_string(); }
                    }
                    "tags" => {
                        if let Some(b) = cur.as_mut() {
                            b.tags = v.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
                        }
                    }
                    _ => {}
                }
            } else if let Some(b) = cur.as_mut() {
                // A non-field line inside a block = summary/title.
                if b.summary.is_empty() {
                    b.summary = line.trim().to_string();
                }
            }
        }
        if let Some(b) = cur.take() { list.bugs.push(b); }
        if list.bugs.is_empty() && saw_any {
            return Err(ComponentError::NoEntries);
        }
        Ok(list)
    }

    /// All bugs.
    pub fn bugs(&self) -> &[Bug] {
        &self.bugs
    }

    /// Bugs at/above a severity.
    pub fn at_least(&self, min: Severity) -> Vec<&Bug> {
        self.bugs.iter().filter(|b| b.severity >= min).collect()
    }

    /// Whether any bug is critical/important.
    pub fn has_serious(&self) -> bool {
        self.bugs.iter().any(|b| b.severity <= Severity::Important)
    }

    /// Number of bugs.
    pub fn len(&self) -> usize {
        self.bugs.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.bugs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUGS: &str = "Bug: #12345\nSeverity: important\nPackage: bash\nTags: patch, security\nBash shell has buffer overflow\n\nBug: #999\nSeverity: minor\nPackage: zlib\nMinor cosmetic issue\n\nBug: #7\nSeverity: critical\nPackage: openssl\nRemote code execution\n";

    #[test]
    fn parses_bug_entries() {
        let l = BugList::parse(BUGS).unwrap();
        assert_eq!(l.len(), 3);
        assert_eq!(l.bugs()[0].number, Some(12345));
        assert_eq!(l.bugs()[0].severity, Severity::Important);
        assert_eq!(l.bugs()[0].package, "bash");
        assert_eq!(l.bugs()[0].tags, vec!["patch", "security"]);
        assert_eq!(l.bugs()[0].summary, "Bash shell has buffer overflow");
    }

    #[test]
    fn severity_filter() {
        let l = BugList::parse(BUGS).unwrap();
        let serious = l.at_least(Severity::Important);
        assert_eq!(serious.len(), 2); // important + critical
        assert!(l.has_serious());
    }

    #[test]
    fn severity_parse() {
        assert_eq!(Severity::parse("critical"), Severity::Critical);
        assert_eq!(Severity::parse("IMPORTANT"), Severity::Important);
        assert_eq!(Severity::parse("bogus"), Severity::Unknown);
    }

    #[test]
    fn empty_ok() {
        assert!(BugList::parse("").unwrap().is_empty());
        assert!(BugList::parse("   \n\n").unwrap().is_empty());
    }
}
