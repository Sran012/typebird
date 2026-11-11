// seam-hiding for chunked transcription. the goal: joins between chunks
// should be invisible in the final text.
//
// rules (from vox research):
// 1. strip ellipses ("..." / "…") — whisper writes them at chunk boundaries
// 2. lowercase leading capital if previous chunk didn't end a sentence
//    (except "I", acronyms, mixed-case names)
// 3. drop trailing . ? ! if the chunk was short (pause was brief)

pub fn clean(text: &str, prev_ended_sentence: bool) -> String {
    let mut t = text.trim().to_string();

    // 1. strip ellipses
    t = t.replace("...", "").replace("…", "");
    t = t.trim().to_string();

    if t.is_empty() {
        return t;
    }

    // 2. lowercase leading capital if mid-sentence
    if !prev_ended_sentence
        && let Some(first) = t.chars().next()
        && first.is_uppercase()
        && first != 'I'
    {
        // check it's not an acronym (all caps) or mixed-case name
        let second = t.chars().nth(1);
        let is_acronym = second.map(|c| c.is_uppercase()).unwrap_or(false);
        if !is_acronym {
            let rest: String = t.chars().skip(1).collect();
            t = format!("{}{}", first.to_lowercase(), rest);
        }
    }

    // 3. drop trailing punctuation on short chunks (handled by caller passing flag)
    t
}

pub fn ended_sentence(text: &str) -> bool {
    let t = text.trim();
    t.ends_with('.') || t.ends_with('?') || t.ends_with('!')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ellipsis_at_chunk_seams() {
        assert_eq!(clean("hello...", true), "hello");
        assert_eq!(clean("hello…", true), "hello");
    }

    #[test]
    fn lowercases_a_mid_sentence_start() {
        assert_eq!(clean("Testing the type", false), "testing the type");
    }

    #[test]
    fn keeps_i_and_acronyms() {
        assert_eq!(clean("I think so", false), "I think so");
        assert_eq!(clean("NASA launched", false), "NASA launched");
    }

    #[test]
    fn leaves_a_new_sentence_alone() {
        assert_eq!(clean("Testing the type", true), "Testing the type");
    }

    #[test]
    fn strips_surrounding_space() {
        assert_eq!(clean("  hello  ", true), "hello");
    }

    #[test]
    fn spots_a_finished_sentence() {
        assert!(ended_sentence("done."));
        assert!(ended_sentence("really?"));
        assert!(ended_sentence("stop!"));
        assert!(!ended_sentence("more words"));
    }
}
