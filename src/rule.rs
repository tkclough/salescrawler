use std::{fmt::{Display, self}, fs};

use base64::Engine;
use serde::{Deserialize, Deserializer, de, de::Visitor, de::MapAccess};
use serde_json::Value;
use sha2::Digest;
use thiserror::Error;

use crate::models::{Post, Title};

#[derive(Deserialize, PartialEq, Default, Debug)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    pub fn read_from_file(filename: &str) -> Result<Self, crate::error::Error> {
        let contents = fs::read_to_string(filename)?;
        let contents: Value = serde_json::from_str(&contents)?;
        let contents = contents
            .as_array()
            .ok_or(crate::error::Error::Other("JSON should be an array".to_owned()))?;
    
        let mut rules: Vec<Rule> = Vec::with_capacity(contents.len());
        for spec in contents.iter() {
            let rule = Rule::parse_json(spec).map_err(crate::error::Error::Rule)?;
            rules.push(rule);
        }
        Ok(Self {
            rules
        })
    }

    pub fn get_matching_rule(&self, post: &Post, title: &Title) -> Option<Rule> {
        for rule in self.rules.iter() {
            if post.is_match(&rule) && title.is_match(&rule) {
                return Some(rule.clone());
            }
        }

        None
    }
}

#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct Rule {
    pub name: Option<String>,
    pub link_flair_pattern: Option<PatternAndSource>,
    pub product_type_pattern: Option<PatternAndSource>,
    pub description_pattern: Option<PatternAndSource>,
    pub price_min_dollars: Option<i64>,
    pub price_max_dollars: Option<i64>,
}

pub trait Subject {
    fn is_match(&self, rule: &Rule) -> bool;
}

impl Rule {
    const fn new() -> Self {
        Self {
            name: None,
            link_flair_pattern: None,
            product_type_pattern: None,
            description_pattern: None,
            price_max_dollars: None,
            price_min_dollars: None,
        }
    }

    pub fn name(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            _ => "(unnamed rule)".to_owned(),
        }
    }

    pub fn parse_json(val: &Value) -> Result<Self, Error> {
        let mut rule = Self::new();

        let val = val.as_object().ok_or(Error::NotAnObject)?;

        let name = val.get("name");
        if let Some(name) = name {
            let name = name.as_str().ok_or_else(|| Error::BadValue("name".to_owned()))?;
            rule.name = Some(name.to_owned());
        }

        let link_flair_pattern = val.get("link_flair_pattern");
        if let Some(link_flair_pattern) = link_flair_pattern {
            let link_flair_pattern = link_flair_pattern
                .as_str()
                .ok_or_else(|| Error::BadValue("link_flair_pattern".to_owned()))?;
            let parsed = parse_pattern(link_flair_pattern)?;
            rule.link_flair_pattern = Some(PatternAndSource { 
                source: link_flair_pattern.to_owned(), 
                pattern: parsed 
            });
        }

        let product_type_pattern = val.get("product_type_pattern");
        if let Some(product_type_pattern) = product_type_pattern {
            let product_type_pattern = product_type_pattern
                .as_str()
                .ok_or_else(|| Error::BadValue("product_type_pattern".to_owned()))?;
            let parsed = parse_pattern(product_type_pattern)?;
            rule.product_type_pattern = Some(PatternAndSource { 
                source: product_type_pattern.to_owned(), 
                pattern: parsed 
            });
        }

        let description_pattern = val.get("description_pattern");
        if let Some(description_pattern) = description_pattern {
            let description_pattern = description_pattern
                .as_str()
                .ok_or_else(|| Error::BadValue("description_pattern".to_owned()))?;
            let parsed = parse_pattern(description_pattern)?;
            rule.description_pattern = Some(PatternAndSource { 
                source: description_pattern.to_owned(), 
                pattern: parsed 
            });
        }

        let price_min = val.get("price_min");
        if let Some(price_min) = price_min {
            let price_min = price_min
                .as_i64()
                .ok_or_else(|| Error::BadValue("price_min".to_owned()))?;
            rule.price_min_dollars = Some(price_min);
        }

        let price_max = val.get("price_max");
        if let Some(price_max) = price_max {
            let price_max = price_max
                .as_i64()
                .ok_or_else(|| Error::BadValue("price_max".to_owned()))?;
            rule.price_max_dollars = Some(price_max);
        }

        Ok(rule)
    }

    pub fn hash(&self) -> String {
        let mut hasher = md5::Md5::new();
        if let Some(name) = &self.name {
            hasher.update(name);
        }
        if let Some(link_flair_pattern) = &self.link_flair_pattern {
            hasher.update(link_flair_pattern.pattern.hash());
        }
        if let Some(product_type_pattern) = &self.product_type_pattern {
            hasher.update(product_type_pattern.pattern.hash());
        }
        if let Some(description_pattern) = &self.description_pattern {
	        hasher.update(description_pattern.pattern.hash());
        }
        if let Some(price_min_dollars) = self.price_min_dollars {
            let payload: &[u8] = bytemuck::bytes_of(&price_min_dollars);
	        hasher.update(payload);
        }
        if let Some(price_max_dollars) = self.price_max_dollars {
            let payload: &[u8] = bytemuck::bytes_of(&price_max_dollars);
	        hasher.update(payload);
        }
        
        base64::engine::general_purpose::STANDARD.encode(hasher.finalize().to_vec())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PatternAndSource {
    pub source: String,
    pub pattern: Pattern,
}

impl<'de> Deserialize<'de> for PatternAndSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PatternAndSourceVisitor;
        impl<'de> Visitor<'de> for PatternAndSourceVisitor {
            type Value = PatternAndSource;

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: de::Error, {
                let mut scanner = Scanner::new(v);
                let pattern = scanner.pattern()
                    .map_err(|e| de::Error::custom(format!("failed to parse pattern: {e}")))?;

                Ok(PatternAndSource { source: v.to_owned(), pattern: pattern })
            }

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("expected a string")
            }
        }

        deserializer.deserialize_str(PatternAndSourceVisitor)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Pattern {
    Exact(String),
    Or(Box<Pattern>, Box<Pattern>),
    And(Box<Pattern>, Box<Pattern>),
    Not(Box<Pattern>),
}

impl Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact(s) => f.write_fmt(format_args!("\"{s}\"")),
            Self::Or(p1, p2) => 
                f.write_fmt(format_args!("{p1} || {p2}")),
            Self::And(p1, p2) =>
                f.write_fmt(format_args!("{p1} && {p2}")),
            Self::Not(p) => 
                f.write_fmt(format_args!("!{p}")),
        }
    }
}

impl Pattern {
    pub fn does_string_match(&self, s: &str) -> bool {
        match self {
            Self::Exact(kwd) => s.to_lowercase().contains(&kwd.to_lowercase()),
            Self::Or(p1, p2) => p1.does_string_match(s) || p2.does_string_match(s),
            Self::And(p1, p2) => p1.does_string_match(s) && p2.does_string_match(s),
            Self::Not(p) => !p.does_string_match(s),
        }
    }

    pub fn does_string_option_match(&self, s: &Option<String>) -> bool {
        match s {
            Some(s) => self.does_string_match(s),
            _ => matches!(self, Pattern::Not(_))
        }
    }

    pub fn hash(&self) -> Vec<u8> {
        let mut hasher = md5::Md5::new();
        match self {
            Pattern::Exact(s) => {
                hasher.update(s);
            },
            Pattern::Or(p1, p2) => {
                hasher.update("||");
                hasher.update(p1.hash());
                hasher.update(p2.hash());
            },
            Pattern::And(p1, p2) => {
                hasher.update("&&");
                hasher.update(p1.hash());
                hasher.update(p2.hash());
            }
            Pattern::Not(p) => {
                hasher.update("!");
                hasher.update(p.hash());
            }
        }
        hasher.finalize().to_vec()
    }
}


// Patterns have the following grammar:
// Pattern ::= <Keyword>
//           | ( Pattern )
//           | <Pattern> || <Pattern>
//           | <Pattern> && <Pattern>
//           | ! <Pattern>
// <Keyword> ::= \w+
//             | \"[^"]+\"
//
// Unambiguous version
// <Pattern> ::= <Factor> <Pattern'>
// <Pattern'> ::= '||' <Factor> <Pattern'>
//              | '&&' <Factor> <Pattern'>
//              | epsilon
// <Factor> ::= '(' <Pattern> ')'
//            | <Keyword>
//            | '!' <Pattern>

#[derive(Debug, PartialEq, Eq)]
enum Token {
    ParenOpen,
    ParenClose,
    OpAnd,
    OpOr,
    OpNegate,
    Keyword(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParenOpen => f.write_str("ParenOpen"),
            Self::ParenClose => f.write_str("ParenClose"),
            Self::OpAnd => f.write_str("OpAnd"),
            Self::OpOr => f.write_str("OpOr"),
            Self::OpNegate => f.write_str("OpNegate"),
            Self::Keyword(kwd) => f.write_fmt(format_args!("Keyword({kwd})")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error("column {0}: expected one of tokens {1}, got token: {2}")]
    ExpectedButGotToken(usize, Tokens, MaybeToken),
    #[error("column {0}: expected one of chars {1}, got char: {2}")]
    ExpectedButGotChar(usize, String, MaybeChar),
    #[error("column {0}: expected non-whitespace character, got: '{1}'")]
    ExpectedNonWhitespace(usize, MaybeChar),
    #[error("column {0}: unexpected character {1}")]
    InvalidKeywordChar(usize, char),
    #[error("column {0}: empty keyword, check quotes")]
    EmptyKeyword(usize),
    #[error("column {0}: can't rewind token because it's null")]
    CantRewindToken(usize),

    #[error("not a json object")]
    NotAnObject,
    #[error("wrong type at key {0}")]
    BadValue(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Tokens(Vec<Token>);

impl Display for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        if self.0.len() > 1 {
            f.write_fmt(format_args!("{}", self.0[0]))?;
        }
        for i in 1..self.0.len() {
            f.write_fmt(format_args!(", {}", self.0[i]))?;
        }
        f.write_str("]")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MaybeToken(Option<Token>);
impl Display for MaybeToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self(Some(t)) => t.fmt(f),
            _ => f.write_str("<<EOF>>"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MaybeChar(Option<char>);
impl Display for MaybeChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self(Some(t)) => t.fmt(f),
            _ => f.write_str("<<EOF>>"),
        }
    }
}

fn parse_pattern(input: &str) -> Result<Pattern, Error> {
    let mut scanner = Scanner::new(input);
    scanner.pattern()
}

#[derive(Debug)]
struct Scanner<'a> {
    source: &'a str,
    cursor: usize,
    last_token: Option<usize>,
}

impl<'a> Scanner<'a> {
    const fn new(source: &str) -> Scanner {
        Scanner {
            source,
            cursor: 0,
            last_token: None,
        }
    }

    fn pattern(&mut self) -> Result<Pattern, Error> {
        let f = self.factor()?;
        let partial_pattern = self.pattern_tail()?;

        match partial_pattern {
            Some(partial_pattern) => Ok(partial_pattern.apply(f)),
            _ => Ok(f),
        }
    }

    fn pattern_tail(&mut self) -> Result<Option<PartialPattern>, Error> {
        let tok = self.next_token()?;
        let is_and = match tok {
            Some(Token::OpAnd) => true,
            Some(Token::OpOr) => false,
            _ => {
                self.rewind_cursor()?;
                return Ok(None);
            }
        };

        let f = self.factor()?;
        let rhs = self.pattern_tail()?.map(Box::new);
        if is_and {
            Ok(Some(PartialPattern::And(f, rhs)))
        } else {
            Ok(Some(PartialPattern::Or(f, rhs)))
        }
    }

    fn factor(&mut self) -> Result<Pattern, Error> {
        let tok = self.next_token()?;
        match tok {
            Some(Token::ParenOpen) => {
                let pat = self.pattern()?;
                let tok = self.next_token()?;
                match tok {
                    Some(Token::ParenClose) => Ok(pat),
                    _ => Err(Error::ExpectedButGotToken(
                        self.cursor,
                        Tokens(vec![Token::ParenClose]),
                        MaybeToken(None),
                    )),
                }
            }
            Some(Token::OpNegate) => {
                let pat = self.pattern()?;
                Ok(Pattern::Not(Box::new(pat)))
            }
            Some(Token::Keyword(kwd)) => Ok(Pattern::Exact(kwd)),
            tok => Err(Error::ExpectedButGotToken(
                self.cursor,
                Tokens(vec![Token::ParenOpen, Token::Keyword(String::new())]),
                MaybeToken(tok),
            )),
        }
    }

    fn keyword(&mut self) -> Result<String, Error> {
        if self.take('"') {
            self.until_next_quote()
        } else if self.is_done() {
            Err(Error::ExpectedNonWhitespace(self.cursor, MaybeChar(None)))
        } else {
            let mut kwd = String::with_capacity(10);
            let ch = match self.pop() {
                Some(ch) => {
                    if ch.is_whitespace() {
                        return Err(Error::ExpectedNonWhitespace(
                            self.cursor,
                            MaybeChar(Some(ch)),
                        ));
                    }
                    ch
                }
                _ => {
                    return Err(Error::ExpectedNonWhitespace(self.cursor, MaybeChar(None)));
                }
            };

            kwd.push(ch);
            while let Some(ch) = self.peek() {
                if ch.is_whitespace()
                    || ch == '!'
                    || ch == '|'
                    || ch == '&'
                    || ch == '('
                    || ch == ')'
                {
                    break;
                } else if !ch.is_alphanumeric() {
                    return Err(Error::InvalidKeywordChar(self.cursor, ch));
                }

                kwd.push(ch);
                self.pop();
            }

            Ok(kwd)
        }
    }

    fn until_next_quote(&mut self) -> Result<String, Error> {
        if self.take('"') {
            return Err(Error::EmptyKeyword(self.cursor));
        }

        let mut kwd = String::with_capacity(10);
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.pop();
                break;
            }

            kwd.push(ch);
            self.pop();
        }

        Ok(kwd)
    }

    fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.cursor)
    }

    fn pop(&mut self) -> Option<char> {
        let ch = self.source.chars().nth(self.cursor);
        self.cursor += 1;
        ch
    }

    fn next_token(&mut self) -> Result<Option<Token>, Error> {
        // Skip whitespace
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.pop();
            } else {
                break;
            }
        }

        self.last_token = Some(self.cursor);
        match self.peek() {
            Some('(') => {
                self.pop();
                Ok(Some(Token::ParenOpen))
            }
            Some(')') => {
                self.pop();
                Ok(Some(Token::ParenClose))
            }
            Some('!') => {
                self.pop();
                Ok(Some(Token::OpNegate))
            }
            Some('|') => {
                self.pop();
                match self.pop() {
                    Some('|') => Ok(Some(Token::OpOr)),
                    Some(ch) => Err(Error::ExpectedButGotChar(
                        self.cursor,
                        "|".to_owned(),
                        MaybeChar(Some(ch)),
                    )),
                    _ => Err(Error::ExpectedButGotChar(
                        self.cursor,
                        "|".to_owned(),
                        MaybeChar(None),
                    )),
                }
            }
            Some('&') => {
                self.pop();
                match self.pop() {
                    Some('&') => Ok(Some(Token::OpAnd)),
                    Some(ch) => Err(Error::ExpectedButGotChar(
                        self.cursor,
                        "&".to_owned(),
                        MaybeChar(Some(ch)),
                    )),
                    _ => Err(Error::ExpectedButGotChar(
                        self.cursor,
                        "&".to_owned(),
                        MaybeChar(None),
                    )),
                }
            }
            Some(_) => {
                let kwd = self.keyword()?;
                Ok(Some(Token::Keyword(kwd)))
            }
            _ => Ok(None),
        }
    }

    fn rewind_cursor(&mut self) -> Result<(), Error> {
        self.cursor = self.last_token.ok_or(Error::CantRewindToken(self.cursor))?;
        self.last_token = None;
        Ok(())
    }

    fn take(&mut self, ch: char) -> bool {
        let is_match = self.peek().map_or(false, |ch_actual| ch_actual == ch);

        if is_match {
            self.pop();
        }

        is_match
    }

    const fn is_done(&self) -> bool {
        self.cursor == self.source.len()
    }
}

enum PartialPattern {
    And(Pattern, Option<Box<PartialPattern>>),
    Or(Pattern, Option<Box<PartialPattern>>),
}

impl PartialPattern {
    fn apply(self, lhs: Pattern) -> Pattern {
        match self {
            Self::And(rhs, partial) => {
                let lhs = Pattern::And(Box::new(lhs), Box::new(rhs));
                match partial {
                    Some(partial) => partial.apply(lhs),
                    _ => lhs,
                }
            }
            Self::Or(rhs, partial) => {
                let lhs = Pattern::Or(Box::new(lhs), Box::new(rhs));
                match partial {
                    Some(partial) => partial.apply(lhs),
                    _ => lhs,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_unquoted() {
        let mut scanner = Scanner::new("nvidia ");
        let res = scanner.keyword();

        assert!(!res.is_err());
        let kwd = res.unwrap();

        assert_eq!(kwd, "nvidia".to_owned());
        assert_eq!(scanner.cursor, 6)
    }

    #[test]
    fn test_keyword_unquoted_end_paren() {
        let mut scanner = Scanner::new("nvidia)");
        let res = scanner.keyword();

        assert!(!res.is_err());
        let kwd = res.unwrap();

        assert_eq!(kwd, "nvidia".to_owned());
        assert_eq!(scanner.cursor, 6);
    }

    #[test]
    fn test_keyword_quoted() {
        let mut scanner = Scanner::new("\"RTX 3080\"");
        let res = scanner.keyword();

        assert!(!res.is_err());
        let kwd = res.unwrap();

        assert_eq!(kwd, "RTX 3080".to_owned());
        assert_eq!(scanner.cursor, 10);
    }

    #[test]
    fn test_until_next_quote() {
        let mut scanner = Scanner::new("abcd\"");
        let res = scanner.until_next_quote();

        assert!(!res.is_err());
        let kwd = res.unwrap();

        assert_eq!(kwd, "abcd".to_owned());
    }

    #[test]
    fn test_pattern_single_keyword() {
        let mut scanner = Scanner::new("abcd");
        let pattern = scanner.pattern();

        assert_eq!(pattern, Ok(Pattern::Exact("abcd".to_owned())));
    }

    #[test]
    fn test_pattern_and() {
        let mut scanner = Scanner::new("nvidia && rtx");
        let pattern = scanner.pattern();

        assert_eq!(
            pattern,
            Ok(Pattern::And(
                Box::new(Pattern::Exact("nvidia".to_owned())),
                Box::new(Pattern::Exact("rtx".to_owned()))
            ))
        );
    }

    #[test]
    fn test_pattern_or() {
        let mut scanner = Scanner::new("nvidia||rtx");
        let pattern = scanner.pattern();

        assert_eq!(
            pattern,
            Ok(Pattern::Or(
                Box::new(Pattern::Exact("nvidia".to_owned())),
                Box::new(Pattern::Exact("rtx".to_owned()))
            ))
        );
    }

    #[test]
    fn test_pattern_not_or() {
        let mut scanner = Scanner::new("!(bad || expensive)");
        let pattern = scanner.pattern();
        assert_eq!(
            pattern,
            Ok(Pattern::Not(Box::new(Pattern::Or(
                Box::new(Pattern::Exact("bad".to_owned())),
                Box::new(Pattern::Exact("expensive".to_owned()))
            ))))
        );
    }

    #[test]
    fn test_pattern_and_or() {
        let mut scanner = Scanner::new("(nvidia && rtx) || (nvidia && \"gtx 3060 ti\")");
        let pattern = scanner.pattern();

        assert_eq!(
            pattern,
            Ok(Pattern::Or(
                Box::new(Pattern::And(
                    Box::new(Pattern::Exact("nvidia".to_owned())),
                    Box::new(Pattern::Exact("rtx".to_owned()))
                )),
                Box::new(Pattern::And(
                    Box::new(Pattern::Exact("nvidia".to_owned())),
                    Box::new(Pattern::Exact("gtx 3060 ti".to_owned()))
                ))
            ))
        );
    }

    #[test]
    fn test_from_json() {
        let json = 
            r#"{
            "name": "test",
            "product_type_pattern": "GPU",
            "description_pattern": "nvidia",
            "price_max_dollars": 1500
        }"#;

        // let parsed = Rule::parse_json(&json);
        let parsed: Result<Rule, _> = serde_json::from_str(json);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();

        assert_eq!(
            parsed,
            Rule {
                name: Some("test".to_owned()),
                link_flair_pattern: None,
                product_type_pattern: Some(PatternAndSource{
                    source: "GPU".to_owned(),
                    pattern: Pattern::Exact("GPU".to_owned())
                }),
                description_pattern: Some(PatternAndSource {
                    source: "nvidia".to_owned(),
                    pattern: Pattern::Exact("nvidia".to_owned())
                }),
                price_min_dollars: None,
                price_max_dollars: Some(1500)
            }
        )
    }
}
