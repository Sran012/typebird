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
    if !prev_ended_sentence {
        if let Some(first) = t.chars().next() {
            if first.is_uppercase() && first != 'I' {
                // check it's not an acronym (all caps) or mixed-case name
                let second = t.chars().nth(1);
                let is_acronym = second.map(|c| c.is_uppercase()).unwrap_or(false);
                if !is_acronym {
                    let rest: String = t.chars().skip(1).collect();
                    t = format!("{}{}", first.to_lowercase(), rest);
                }
            }
        }
    }

    // 3. drop trailing punctuation on short chunks (handled by caller passing flag)
    t
}

pub fn ended_sentence(text: &str) -> bool {
    let t = text.trim();
    t.ends_with('.') || t.ends_with('?') || t.ends_with('!')
}
