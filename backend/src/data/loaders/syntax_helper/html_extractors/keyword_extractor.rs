//! Извлечение ключевых слов из справки по языку (shlang_ru)

use scraper::{Html, Selector};

pub struct KeywordExtractor;

impl KeywordExtractor {
    pub fn extract_keywords(document: &Html) -> Vec<String> {
        let mut keywords = Vec::new();
        Self::extract_with_selector(document, "strong.ControlElement", TagKind::Keyword, &mut keywords);
        Self::extract_with_selector(document, "u", TagKind::Keyword, &mut keywords);
        Self::extract_with_selector(document, "span.SourceCode", TagKind::Directive, &mut keywords);

        keywords
    }

    fn extract_with_selector(
        document: &Html,
        selector_str: &str,
        kind: TagKind,
        keywords: &mut Vec<String>,
    ) {
        let Ok(selector) = Selector::parse(selector_str) else {
            return;
        };
        for element in document.select(&selector) {
            let text = element.text().collect::<String>();
            Self::collect_tokens(&text, kind, keywords);
        }
    }

    fn collect_tokens(text: &str, kind: TagKind, keywords: &mut Vec<String>) {
        let normalized = text.replace('\u{a0}', " ");
        if normalized.contains("//") {
            return;
        }
        for token in normalized.split_whitespace() {
            let Some(keyword) = Self::normalize_token(token) else {
                continue;
            };
            match kind {
                TagKind::Keyword => {
                    keywords.push(keyword);
                }
                TagKind::Directive => {
                    if keyword.starts_with('#') || keyword.starts_with('&') {
                        keywords.push(keyword);
                    }
                }
            }
        }
    }

    fn normalize_token(token: &str) -> Option<String> {
        let trimmed = token.trim_matches(|c: char| {
            matches!(
                c,
                ';' | ',' | '.' | ':' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''
                    | '—' | '-' | '/'
            )
        });
        let trimmed = trimmed.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.contains('<') || trimmed.contains('>') {
            return None;
        }

        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '#' || c == '&')
        {
            return None;
        }

        if !trimmed
            .chars()
            .any(|c| c.is_alphabetic() || c == '#' || c == '&')
        {
            return None;
        }

        Some(trimmed.to_string())
    }
}

#[derive(Clone, Copy)]
enum TagKind {
    Keyword,
    Directive,
}
