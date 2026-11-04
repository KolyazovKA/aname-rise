// история буфера обмена без зависимостей от SDK: чистая логика, её можно тестировать отдельно

pub const MAX_ENTRIES: usize = 50;
// слишком большие тексты не храним: 50 записей по сотне килобайт раздули бы память
pub const MAX_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub struct Store {
    // новые записи в начале
    items: Vec<String>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    // false, если запись отброшена (пустая или слишком большая)
    pub fn push(&mut self, text: &str) -> bool {
        if text.trim().is_empty() || text.len() > MAX_BYTES {
            return false;
        }

        // дубликат переезжает наверх, а не копится
        self.items.retain(|item| item != text);
        self.items.insert(0, text.to_string());
        self.items.truncate(MAX_ENTRIES);

        true
    }

    // записи, где встречаются все слова запроса (без учёта регистра), новые первыми
    pub fn filter(&self, query: &str) -> Vec<&str> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();

        self.items
            .iter()
            .filter(|item| {
                if words.is_empty() {
                    return true;
                }

                let haystack = item.to_lowercase();

                words.iter().all(|word| haystack.contains(word.as_str()))
            })
            .map(String::as_str)
            .collect()
    }
}

// одна строка для списка: переводы строк и повторные пробелы схлопываются, длинное обрезается
pub fn preview(text: &str, max_chars: usize) -> String {
    // читаем только начало: превью длинного текста не должно обходить всё содержимое
    let head: String = text.chars().take(max_chars * 4).collect();
    let line = head.split_whitespace().collect::<Vec<_>>().join(" ");

    if line.chars().count() <= max_chars {
        return line;
    }

    let mut short: String = line.chars().take(max_chars.saturating_sub(1)).collect();

    short.push('…');

    short
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_last_fifty() {
        let mut store = Store::new();

        for number in 0..60 {
            store.push(&format!("text {number}"));
        }

        let all = store.filter("");

        assert_eq!(all.len(), MAX_ENTRIES);
        assert_eq!(all[0], "text 59");
        assert_eq!(all[MAX_ENTRIES - 1], "text 10");
    }

    #[test]
    fn duplicate_moves_to_front() {
        let mut store = Store::new();

        store.push("a");
        store.push("b");
        store.push("a");

        assert_eq!(store.filter(""), vec!["a", "b"]);
    }

    #[test]
    fn rejects_empty_and_huge() {
        let mut store = Store::new();

        assert!(!store.push(""));
        assert!(!store.push("  \n\t "));
        assert!(!store.push(&"x".repeat(MAX_BYTES + 1)));
        assert!(store.push(&"x".repeat(MAX_BYTES)));
        assert_eq!(store.filter("").len(), 1);
    }

    #[test]
    fn keeps_text_as_is() {
        let mut store = Store::new();

        store.push("  line one\nline two\n");

        assert_eq!(store.filter("")[0], "  line one\nline two\n");
    }

    #[test]
    fn filter_ignores_case_and_matches_all_words() {
        let mut store = Store::new();

        store.push("Привет, Мир");
        store.push("Hello World");
        store.push("hello there");

        assert_eq!(store.filter("привет мир"), vec!["Привет, Мир"]);
        assert_eq!(store.filter("HELLO"), vec!["hello there", "Hello World"]);
        assert_eq!(store.filter("hello world"), vec!["Hello World"]);
        assert!(store.filter("nothing").is_empty());
    }

    #[test]
    fn preview_collapses_lines_and_trims() {
        assert_eq!(preview("  a\n\n b\tc  ", 20), "a b c");
        assert_eq!(preview("abcdefghij", 5), "abcd…");
        assert_eq!(preview("abcde", 5), "abcde");
    }

    #[test]
    fn preview_cuts_on_chars_not_bytes() {
        assert_eq!(preview("приветмир", 4), "при…");
    }
}