//! Interactive mode helper.

use rustyline::{
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
    Helper,
};

/// A [Helper] implementation.
pub struct RustylineHelper {
    entries: Vec<String>,
}

impl RustylineHelper {
    /// Creates a new helper from the given entries.
    pub fn new(entries: Vec<String>) -> Self {
        Self { entries }
    }
}

impl Hinter for RustylineHelper {
    type Hint = String;
}

impl Highlighter for RustylineHelper {}

impl Validator for RustylineHelper {}

impl Helper for RustylineHelper {}

/// Like `str::split`, but keeps positional information of every chunk.
struct SplitPosition<'a> {
    last_position: usize,
    line: &'a str,
}

impl<'a> Iterator for SplitPosition<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.line.is_empty() {
            return None;
        }
        if let Some((whitespace_position, whitespace_char)) =
            self.line.char_indices().find(|(_, c)| c.is_whitespace())
        {
            let current_position = self.last_position;
            self.last_position += whitespace_position + whitespace_char.len_utf8();

            let (result, remaining) = self.line.split_at(whitespace_position);
            let mut remaining = remaining.chars();
            remaining.next().expect("This is the whitespace");
            self.line = remaining.as_str();
            Some((current_position, result))
        } else {
            let what_remains = std::mem::replace(&mut self.line, "");
            Some((self.last_position, what_remains))
        }
    }
}

fn split_position(line: &str) -> SplitPosition<'_> {
    SplitPosition {
        last_position: 0,
        line,
    }
}

#[test]
fn check_split() {
    let input = "set kek  löl    something-something";
    let mut iter = split_position(input);
    assert_eq!(Some((0, "set")), iter.next());
    assert_eq!(Some((4, "kek")), iter.next());
    assert_eq!(Some((8, "")), iter.next());
    assert_eq!(Some((9, "löl")), iter.next());
    assert_eq!(Some((14, "")), iter.next()); // "ö" contains two bytes, hence 14, not 13.
    assert_eq!(Some((15, "")), iter.next());
    assert_eq!(Some((16, "")), iter.next());
    assert_eq!(Some((17, "something-something")), iter.next());
    assert_eq!(None, iter.next());

    let input = "set ";
    let mut iter = split_position(input);
    assert_eq!(Some((0, "set")), iter.next());
    assert_eq!(None, iter.next());
}

impl Completer for RustylineHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut items = split_position(line).filter(|(_, part)| !part.is_empty());
        let cmd = match items.next() {
            Some((_index, item)) => item,
            None => {
                // No input yet
                return Ok((
                    0,
                    vec![
                        Pair {
                            display: "set — set oneshot entry".into(),
                            replacement: "set".into(),
                        },
                        Pair {
                            display: "exit — exit the application".into(),
                            replacement: "exit".into(),
                        },
                    ],
                ));
            }
        };
        if pos < 3 && "set".starts_with(line) {
            return Ok((
                0,
                vec![Pair {
                    display: "set — set oneshot entry".into(),
                    replacement: "set".into(),
                }],
            ));
        } else if pos < 4 && "exit".starts_with(line) {
            return Ok((
                0,
                vec![Pair {
                    display: "exit — exit the application".into(),
                    replacement: "exit".into(),
                }],
            ));
        }
        match cmd {
            "set" => { /* No op */ }
            "exit" => {
                // No arguments expected
                return Ok((0, vec![]));
            }
            _ => {
                // Unknown argument
                return Ok((0, vec![]));
            }
        }

        if let Some((index, partial_entry)) = items.next() {
            // User has begun to input an entry.
            if let Some(relative_pos) = pos.checked_sub(index) {
                if relative_pos == 0 {
                    // Show all entries
                    return Ok((
                        4,
                        self.entries
                            .iter()
                            .map(|entry| Pair {
                                display: entry.clone(),
                                replacement: entry.clone(),
                            })
                            .collect(),
                    ));
                }
                if relative_pos <= partial_entry.len() {
                    return Ok((
                        4,
                        self.entries
                            .iter()
                            .filter(|entry| entry.starts_with(partial_entry))
                            .map(|entry| Pair {
                                display: entry.clone(),
                                replacement: entry.clone(),
                            })
                            .collect(),
                    ));
                }
            }
        } else {
            // No entry yet!
            return Ok((
                3,
                self.entries
                    .iter()
                    .map(|entry| Pair {
                        display: entry.clone(),
                        replacement: format!(" {}", entry),
                    })
                    .collect(),
            ));
        }

        let _ = (line, pos, ctx);
        Ok((0, Vec::with_capacity(0)))
    }

    fn update(&self, line: &mut rustyline::line_buffer::LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}
