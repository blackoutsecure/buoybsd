// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
enum Token {
    Star,
    Any,
    Literal(char),
    Class {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}

#[derive(Clone, Debug)]
pub struct Glob(Vec<Token>);

impl Glob {
    pub fn compile(pattern: &str) -> Result<Self> {
        let bad = || Error::message("invalid glob selector");
        let mut chars = pattern.chars().peekable();
        let mut tokens = Vec::new();
        while let Some(c) = chars.next() {
            tokens.push(match c {
                '*' => {
                    if matches!(tokens.last(), Some(Token::Star)) {
                        continue;
                    }
                    Token::Star
                }
                '?' => Token::Any,
                '\\' => Token::Literal(chars.next().ok_or_else(bad)?),
                '[' => {
                    let negated = chars.next_if_eq(&'^').is_some();
                    let mut ranges = Vec::new();
                    fn endpoint(
                        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
                    ) -> Option<char> {
                        match chars.next()? {
                            '\\' => chars.next(),
                            ']' | '-' => None,
                            c => Some(c),
                        }
                    }
                    loop {
                        let lo = endpoint(&mut chars).ok_or_else(bad)?;
                        let hi = if chars.next_if_eq(&'-').is_some() {
                            endpoint(&mut chars).ok_or_else(bad)?
                        } else {
                            lo
                        };
                        ranges.push((lo, hi));
                        if chars.next_if_eq(&']').is_some() {
                            break;
                        }
                        if chars.peek().is_none() {
                            return Err(bad());
                        }
                    }
                    Token::Class { negated, ranges }
                }
                c => Token::Literal(c),
            });
        }
        Ok(Self(tokens))
    }

    pub fn matches(&self, observed: &str) -> bool {
        let chars: Vec<char> = observed.chars().collect();
        let mut previous = vec![false; chars.len() + 1];
        previous[0] = true;
        for token in &self.0 {
            let mut next = vec![false; chars.len() + 1];
            match token {
                Token::Star => {
                    next[0] = previous[0];
                    for (i, c) in chars.iter().enumerate() {
                        next[i + 1] = previous[i + 1] || (next[i] && *c != '/');
                    }
                }
                _ => {
                    for (i, c) in chars.iter().enumerate() {
                        let matched = match token {
                            Token::Any => *c != '/',
                            Token::Literal(value) => c == value,
                            Token::Class { negated, ranges } => {
                                ranges.iter().any(|(lo, hi)| lo <= c && c <= hi) != *negated
                            }
                            Token::Star => unreachable!(),
                        };
                        next[i + 1] = previous[i] && matched;
                    }
                }
            }
            previous = next;
        }
        previous[chars.len()]
    }
}
