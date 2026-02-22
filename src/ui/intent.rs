use crate::ui::catalog::UiIntent;
use std::collections::BTreeSet;

pub fn intent_from_text(text: &str) -> Option<UiIntent> {
    let lowered = text.to_ascii_lowercase();
    let tokens = token_set(&lowered);
    let has = |term: &str| tokens.contains(term);
    let has_any_phrase = |phrases: &[&str]| phrases.iter().any(|phrase| lowered.contains(phrase));

    let mentions_files = has("file") || has("files");
    let mentions_workspace = has("workspace");
    let asks_file_visibility = has("show")
        || has("list")
        || has("display")
        || has("browse")
        || has("view")
        || lowered.starts_with("what files");

    let primary = if has_any_phrase(&[
        "list files",
        "listing of files",
        "file tree",
        "directory tree",
        "show files",
        "show me files",
        "show the files",
        "all the files",
        "all files",
        "workspace files",
    ]) || (mentions_files && has("canvas"))
        || (mentions_files && mentions_workspace && asks_file_visibility)
    {
        "file_listing".to_string()
    } else if has("plan") || has("roadmap") || has("milestone") {
        "plan_review".to_string()
    } else if has("ui") && has("design") {
        "ui_design_review".to_string()
    } else if has("review")
        || has("approve")
        || has("reject")
        || has("decline")
        || has("spec")
        || has("diff")
        || has("patch")
        || has("security")
    {
        "code_review".to_string()
    } else {
        return None;
    };

    let mut operations = BTreeSet::new();
    if has("approve") {
        operations.insert("approve".to_string());
    }
    if has("reject") || has("decline") {
        operations.insert("reject".to_string());
    }
    if has("revise") || has("change") {
        operations.insert("revise".to_string());
    }
    if primary == "file_listing" {
        if has("browse") {
            operations.insert("browse".to_string());
        }
        if has("view") {
            operations.insert("view".to_string());
        }
        if has("show") || has("list") || has("display") {
            operations.insert("list".to_string());
        }
    }
    if operations.is_empty() {
        if primary == "file_listing" {
            operations.insert("list".to_string());
        } else if primary == "code_review" {
            operations.insert("review".to_string());
        }
    }

    let mut tags = BTreeSet::new();
    if has("spec") {
        tags.insert("spec".to_string());
    }
    if has("diff") || has("patch") {
        tags.insert("diff".to_string());
    }
    if has("security") {
        tags.insert("security".to_string());
    }
    if has("plan") || has("roadmap") {
        tags.insert("plan".to_string());
    }
    if primary == "file_listing" {
        tags.insert("files".to_string());
        if mentions_workspace {
            tags.insert("workspace".to_string());
        }
        if has("tree") || has("directory") {
            tags.insert("tree".to_string());
        }
    }

    Some(UiIntent::new(
        primary,
        operations.into_iter().collect(),
        tags.into_iter().collect(),
    ))
}

fn token_set(text: &str) -> BTreeSet<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::intent_from_text;

    #[test]
    fn detects_workspace_file_request_with_articles() {
        let intent = intent_from_text("Show the files in the workspace")
            .expect("intent should be detected for workspace file listing");
        assert_eq!(intent.primary, "file_listing");
        assert!(intent.operations.contains(&"list".to_string()));
        assert!(intent.tags.contains(&"files".to_string()));
        assert!(intent.tags.contains(&"workspace".to_string()));
    }

    #[test]
    fn detects_workspace_file_request_without_show_phrase() {
        let intent = intent_from_text("workspace files please")
            .expect("intent should be detected for workspace file listing");
        assert_eq!(intent.primary, "file_listing");
    }

    #[test]
    fn detects_code_review_intent() {
        let intent = intent_from_text("review this patch for security risks")
            .expect("intent should be detected for code review");
        assert_eq!(intent.primary, "code_review");
        assert!(intent.tags.contains(&"diff".to_string()));
        assert!(intent.tags.contains(&"security".to_string()));
    }

    #[test]
    fn returns_none_for_non_ui_prompt() {
        assert!(intent_from_text("hello there").is_none());
    }
}
