use std::fmt;

enum ChangeTag {
    Added,
    Removed,
    Equal,
}

pub struct DiffResult {
    lines: Vec<(ChangeTag, String, Option<usize>, Option<usize>)>,
}

impl fmt::Display for DiffResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (tag, line, old_index, new_index) in &self.lines {
            use colored::Colorize;

            let line_nums = match (old_index, new_index) {
                (Some(old), Some(new)) => format!("{old:>2} {new:>2}"),
                (Some(old), None) => format!("{old:>2}   "),
                (None, Some(new)) => format!("   {new:>2}"),
                (None, None) => unreachable!("Both indices are None"),
            };

            match tag {
                ChangeTag::Added => write!(f, "{}", format!("{line_nums} + {line}").green())?,
                ChangeTag::Removed => write!(f, "{}", format!("{line_nums} - {line}").red())?,
                ChangeTag::Equal => write!(f, "{line_nums}   {line}")?,
            }
        }
        Ok(())
    }
}

pub fn build_diff(a: &str, b: &str) -> DiffResult {
    let mut diff = DiffResult { lines: Vec::new() };
    let text_diff = similar::TextDiff::from_lines(a, b);

    for change in text_diff.iter_all_changes() {
        let tag = match change.tag() {
            similar::ChangeTag::Delete => ChangeTag::Removed,
            similar::ChangeTag::Insert => ChangeTag::Added,
            similar::ChangeTag::Equal => ChangeTag::Equal,
        };

        let old_index = change.old_index();
        let new_index = change.new_index();

        diff.lines
            .push((tag, change.to_string(), old_index, new_index));
    }

    diff
}
