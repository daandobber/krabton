//! Lyrics as voicebanks spell them: romaji turned into hiragana (Japanese
//! voicebanks name their samples in kana), and the vowel a syllable ends on
//! (a VCV voicebank's next sample starts from it).

/// Romaji syllables and their hiragana, longest first within each start.
const TABLE: &[(&str, &str)] = &[
    ("kya", "きゃ"), ("kyu", "きゅ"), ("kyo", "きょ"), ("sha", "しゃ"), ("shu", "しゅ"), ("sho", "しょ"), ("she", "しぇ"),
    ("cha", "ちゃ"), ("chu", "ちゅ"), ("cho", "ちょ"), ("che", "ちぇ"), ("nya", "にゃ"), ("nyu", "にゅ"), ("nyo", "にょ"),
    ("hya", "ひゃ"), ("hyu", "ひゅ"), ("hyo", "ひょ"), ("mya", "みゃ"), ("myu", "みゅ"), ("myo", "みょ"), ("rya", "りゃ"),
    ("ryu", "りゅ"), ("ryo", "りょ"), ("gya", "ぎゃ"), ("gyu", "ぎゅ"), ("gyo", "ぎょ"), ("bya", "びゃ"), ("byu", "びゅ"),
    ("byo", "びょ"), ("pya", "ぴゃ"), ("pyu", "ぴゅ"), ("pyo", "ぴょ"), ("tsu", "つ"), ("shi", "し"), ("chi", "ち"),
    ("ja", "じゃ"), ("ju", "じゅ"), ("jo", "じょ"), ("je", "じぇ"), ("ji", "じ"), ("fu", "ふ"), ("fa", "ふぁ"), ("fi", "ふぃ"),
    ("fe", "ふぇ"), ("fo", "ふぉ"), ("ti", "てぃ"), ("di", "でぃ"), ("tu", "とぅ"), ("du", "どぅ"), ("wi", "うぃ"), ("we", "うぇ"),
    ("wo", "を"), ("va", "ゔぁ"), ("vi", "ゔぃ"), ("vu", "ゔ"), ("ve", "ゔぇ"), ("vo", "ゔぉ"),
    ("ka", "か"), ("ki", "き"), ("ku", "く"), ("ke", "け"), ("ko", "こ"), ("ga", "が"), ("gi", "ぎ"), ("gu", "ぐ"), ("ge", "げ"), ("go", "ご"),
    ("sa", "さ"), ("si", "し"), ("su", "す"), ("se", "せ"), ("so", "そ"), ("za", "ざ"), ("zi", "じ"), ("zu", "ず"), ("ze", "ぜ"), ("zo", "ぞ"),
    ("ta", "た"), ("te", "て"), ("to", "と"), ("da", "だ"), ("de", "で"), ("do", "ど"),
    ("na", "な"), ("ni", "に"), ("nu", "ぬ"), ("ne", "ね"), ("no", "の"), ("ha", "は"), ("hi", "ひ"), ("hu", "ふ"), ("he", "へ"), ("ho", "ほ"),
    ("ba", "ば"), ("bi", "び"), ("bu", "ぶ"), ("be", "べ"), ("bo", "ぼ"), ("pa", "ぱ"), ("pi", "ぴ"), ("pu", "ぷ"), ("pe", "ぺ"), ("po", "ぽ"),
    ("ma", "ま"), ("mi", "み"), ("mu", "む"), ("me", "め"), ("mo", "も"), ("ya", "や"), ("yu", "ゆ"), ("yo", "よ"),
    ("ra", "ら"), ("ri", "り"), ("ru", "る"), ("re", "れ"), ("ro", "ろ"), ("la", "ら"), ("li", "り"), ("lu", "る"), ("le", "れ"), ("lo", "ろ"),
    ("wa", "わ"), ("a", "あ"), ("i", "い"), ("u", "う"), ("e", "え"), ("o", "お"), ("n", "ん"),
];

/// A romaji syllable (or several) in hiragana; anything else as it was.
pub fn to_kana(lyric: &str) -> String {
    let s = lyric.trim().to_lowercase();
    if s.is_empty() || !s.is_ascii() {
        return lyric.trim().to_string();
    }
    let mut out = String::new();
    let mut rest = s.as_str();
    'next: while !rest.is_empty() {
        for (r, k) in TABLE {
            if rest.starts_with(r) {
                out.push_str(k);
                rest = &rest[r.len()..];
                continue 'next;
            }
        }
        // A doubled consonant: a small tsu.
        let mut ch = rest.chars();
        let c = ch.next().unwrap_or(' ');
        if c.is_ascii_alphabetic() && ch.next() == Some(c) && !"aeiou".contains(c) {
            out.push('っ');
            rest = &rest[1..];
            continue;
        }
        return lyric.trim().to_string();
    }
    out
}

/// A word's romaji syllables ("sakura": sa ku ra; a doubled consonant
/// dropped, "kitte": ki te), or None when it is not romaji.
fn romaji_syllables(word: &str) -> Option<Vec<String>> {
    let s = word.to_lowercase();
    let mut out = Vec::new();
    let mut rest = s.as_str();
    'next: while !rest.is_empty() {
        for (r, _) in TABLE {
            if rest.starts_with(r) {
                out.push(r.to_string());
                rest = &rest[r.len()..];
                continue 'next;
            }
        }
        let mut ch = rest.chars();
        let c = ch.next()?;
        if c.is_ascii_alphabetic() && ch.next() == Some(c) && !"aeiou".contains(c) {
            rest = &rest[1..];
            continue;
        }
        return None;
    }
    Some(out)
}

/// Characters that show as nothing (or as a box): zero-width spaces and
/// joiners, soft hyphens, byte order marks, control characters. Text copied
/// from a web page often carries them.
pub fn invisible(c: char) -> bool {
    c.is_control() || matches!(c, '\u{00AD}' | '\u{180E}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}')
}

/// Lyrics typed as a line, a syllable each: words split on spaces and
/// hyphens ("hap-py"), romaji words into their syllables, kana into their
/// morae (small kana and the long vowel mark staying with the one before);
/// a word that is neither stays whole. Invisible characters are left out.
pub fn split_lyrics(text: &str) -> Vec<String> {
    let small = "ゃゅょぁぃぅぇぉゎャュョァィゥェォヮー";
    let text: String = text.chars().filter(|c| !invisible(*c)).collect();
    let mut out: Vec<String> = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || c == '-' || c == '/' || c == ',').filter(|w| !w.is_empty()) {
        if word.is_ascii() {
            match romaji_syllables(word) {
                Some(v) if !v.is_empty() => out.extend(v),
                _ => out.push(word.to_string()),
            }
        } else {
            let mut started = false;
            for c in word.chars() {
                match out.last_mut() {
                    Some(last) if started && small.contains(c) => last.push(c),
                    _ => out.push(c.to_string()),
                }
                started = true;
            }
        }
    }
    out
}

/// The vowel a syllable ends on ('a', 'i', 'u', 'e', 'o', or 'n'), in kana or romaji.
pub fn vowel_of(syllable: &str) -> Option<char> {
    let s = syllable.trim();
    let last = s.chars().last()?;
    if last.is_ascii() {
        return s.chars().rev().find(|c| "aiueon".contains(*c)).map(|c| if c == 'n' && !s.ends_with('n') { 'a' } else { c });
    }
    const ROWS: [(&str, char); 6] = [
        ("あかさたなはまやらわがざだばぱぁゃゎアカサタナハマヤラワガザダバパァャヮ", 'a'),
        ("いきしちにひみりぎじぢびぴぃイキシチニヒミリギジヂビピィ", 'i'),
        ("うくすつぬふむゆるぐずづぶぷぅゅゔウクスツヌフムユルグズヅブプゥュヴ", 'u'),
        ("えけせてねへめれげぜでべぺぇエケセテネヘメレゲゼデベペェ", 'e'),
        ("おこそとのほもよろをごぞどぼぽぉょオコソトノホモヨロヲゴゾドボポォョ", 'o'),
        ("んン", 'n'),
    ];
    // The long vowel mark keeps the vowel before it.
    let c = s.chars().rev().find(|c| *c != 'ー')?;
    ROWS.iter().find(|(chars, _)| chars.contains(c)).map(|(_, v)| *v)
}

/// The vowel as a VCV voicebank writes it before the next syllable ("a か").
pub fn vowel_name(v: char) -> &'static str {
    match v {
        'a' => "a",
        'i' => "i",
        'u' => "u",
        'e' => "e",
        'o' => "o",
        _ => "n",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romaji_turns_into_kana_and_vowels_are_found() {
        assert_eq!(to_kana("ka"), "か");
        assert_eq!(to_kana("sakura"), "さくら");
        assert_eq!(to_kana("kitte"), "きって");
        assert_eq!(to_kana("shinju"), "しんじゅ");
        assert_eq!(to_kana("あ"), "あ");
        assert_eq!(to_kana("xyz"), "xyz", "not romaji: as it is");
        assert_eq!(vowel_of("か"), Some('a'));
        assert_eq!(vowel_of("きょ"), Some('o'));
        assert_eq!(vowel_of("ん"), Some('n'));
        assert_eq!(vowel_of("ka"), Some('a'));
        assert_eq!(vowel_of("らー"), Some('a'));
    }

    #[test]
    fn lyrics_split_into_syllables() {
        assert_eq!(split_lyrics("sakura sakura"), ["sa", "ku", "ra", "sa", "ku", "ra"]);
        // Words that are not romaji stay whole.
        assert_eq!(split_lyrics("hap-py birth-day"), ["hap", "py", "birth", "day"]);
        assert_eq!(split_lyrics("kitte"), ["ki", "te"]);
        assert_eq!(split_lyrics("きょうはー"), ["きょ", "う", "はー"]);
        assert_eq!(split_lyrics("la la  world"), ["la", "la", "world"]);
        // Copied from a web page: no boxes for the invisible characters, no syllables of them.
        assert_eq!(split_lyrics("\u{feff}さ\u{200b}く ra\u{a0}la"), ["さ", "く", "ra", "la"]);
    }
}
