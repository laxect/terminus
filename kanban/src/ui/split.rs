use std::iter::Peekable;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub(crate) struct Split<'a> {
    str: &'a str,
    graph: Peekable<GraphemeIndices<'a>>,
    len: usize,
}

impl<'a> Split<'a> {
    fn new(str: &'a str, len: usize) -> Self {
        let graph = str.grapheme_indices(true).peekable();
        Self { str, graph, len }
    }
}

impl<'a> Iterator for Split<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        let start = if let Some((ind, _char)) = self.graph.peek() {
            *ind
        } else {
            return None;
        };
        let mut end = start;
        let mut length_now = 0usize;
        loop {
            if let Some((_ind, char)) = self.graph.peek() {
                let char_width = char.width_cjk();
                let length_next = length_now + char_width;
                if length_next > self.len {
                    return Some(&self.str[start..end]);
                } else {
                    end += char.len();
                    length_now = length_next;
                    self.graph.next();
                }
            } else {
                return Some(&self.str[start..end]);
            }
        }
    }
}

pub(crate) trait UnicodeSplit: UnicodeSegmentation {
    fn unicode_split(&self, len: usize) -> Split;
}

impl UnicodeSplit for str {
    fn unicode_split(&self, len: usize) -> Split {
        Split::new(self, len)
    }
}

#[cfg(test)]
mod tests {
    use super::UnicodeSplit;

    #[test]
    fn split_space() {
        let input = "";
        let mut split = input.unicode_split(4);
        assert_eq!(Some(""), split.next());
        assert_eq!(None, split.next());
    }

    #[test]
    fn split_non_cjk() {
        let input = "like a rolling stone";
        let mut split = input.unicode_split(4);
        assert_eq!(Some("like"), split.next());
        assert_eq!(Some(" a r"), split.next());
        assert_eq!(Some("olli"), split.next());
    }

    #[test]
    fn split_cjk() {
        let input = "蓬鬆奇風鳥是補充包1106登場的全新鳥獸族系列";
        let mut split = input.unicode_split(20);
        assert_eq!(Some("蓬鬆奇風鳥是補充包11"), split.next());
        assert_eq!(Some("06登場的全新鳥獸族系"), split.next());
        assert_eq!(Some("列"), split.next());
    }

    #[test]
    fn split_over() {
        let input = "蓬鬆";
        let mut split = input.unicode_split(20);
        assert_eq!(Some("蓬鬆"), split.next());
        assert_eq!(None, split.next());
        assert_eq!(None, split.next());
    }
}
