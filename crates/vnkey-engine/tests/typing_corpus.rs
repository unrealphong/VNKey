//! Types a generated corpus into a simulated text field and compares the
//! final texts with `data/typing-expected.txt`, one line per case in corpus
//! order. Run with `VNKEY_BLESS=1` to rewrite the file after an intended
//! behavior change.

use std::{env, fs, path::PathBuf};

use vnkey_engine::{Config, Engine, InputType, Key, KeyInput, Output};

const ONSETS: [&str; 28] = [
    "", "b", "c", "ch", "d", "đ", "g", "gh", "gi", "h", "k", "kh", "l", "m", "n", "ng", "ngh",
    "nh", "p", "ph", "qu", "r", "s", "t", "th", "tr", "v", "x",
];

const RHYMES: &[&str] = &[
    "a", "ai", "ao", "au", "ay", "am", "an", "ang", "anh", "ap", "at", "ac", "ach", "ăm", "ăn",
    "ăng", "ăp", "ăt", "ăc", "âu", "ây", "âm", "ân", "âng", "âp", "ât", "âc", "e", "eo", "em",
    "en", "eng", "ep", "et", "ec", "ê", "êu", "êm", "ên", "ênh", "êp", "êt", "êch", "i", "ia",
    "iu", "im", "in", "inh", "ip", "it", "ich", "iêu", "iêm", "iên", "iêng", "iêp", "iêt", "iêc",
    "o", "oa", "oai", "oay", "oan", "oang", "oanh", "oăn", "oăng", "oat", "oăt", "oac", "oach",
    "oe", "oen", "oet", "oi", "om", "on", "ong", "op", "ot", "oc", "oong", "ooc", "ô", "ôi", "ôm",
    "ôn", "ông", "ôp", "ôt", "ôc", "ơ", "ơi", "ơm", "ơn", "ơp", "ơt", "u", "ua", "uân", "uâng",
    "uât", "uây", "uê", "uênh", "uêch", "ui", "um", "un", "ung", "up", "ut", "uc", "uy", "uya",
    "uyên", "uyêt", "uyt", "uynh", "uych", "uôi", "uôm", "uôn", "uông", "uôt", "uôc", "uơ", "ư",
    "ưa", "ưi", "ưu", "ưng", "ưt", "ưc", "ưm", "ươi", "ươu", "ươm", "ươn", "ương", "ươp", "ươt",
    "ươc", "y", "yêu", "yên", "yêm", "yêt",
];

const ENGLISH: &[&str] = &[
    "class", "mass", "boss", "miss", "press", "hello", "world", "window", "file", "test", "code",
    "video", "radio", "tax", "fix", "box", "next", "text", "java", "rust", "program", "computer",
    "google", "facebook", "office", "coffee", "free", "tree", "see", "too", "zoo", "wow", "new",
    "show", "know", "flow", "law", "raw", "saw", "draw", "jaw", "row", "cow", "now", "how", "we",
    "web", "awesome", "sweet", "issue", "assess", "success", "access", "process", "address",
    "express", "bless", "chess", "guess", "less", "dress", "loss", "cross", "toss", "grass",
    "pass", "jazz", "quiz", "fizz", "buzz", "dd", "xx", "ss", "ff", "rr", "jj", "aa", "ee", "oo",
    "ww", "Windows", "iPhone", "GitHub", "README", "user", "users", "email", "server", "data",
    "string", "return", "function", "default", "struct", "window", "screen", "mouse", "keyboard",
];

const EDGE_CASES: &[&str] = &[
    "tienge",
    "aaa",
    "ss",
    "tieengss",
    "tw",
    "quow",
    "thuowngf",
    "gif",
    "quaf",
    "gioongs",
    "nguyeenx",
    "Vieejt",
    "DDuowngf",
    "khoong",
    "tieeng<BS>s",
    "tie<CLICK>e",
    "chuwa dc",
    "Tooi laf nguwowif Vieetj Nam.",
    "xin chaof caacs banj",
    "vieetj<BS><BS>t",
    "tieengs <BS>a",
    "aw<BS>w",
    "hoaf hoas hoar hoax hoaj",
    "thuyeenf khuyeen",
    "uwow uow",
    "nguwowif",
    "dduwowcj",
    "ddd",
    "www",
    "w",
    "W",
    "[",
    "]",
    "{",
    "}",
    "tw[",
    "t]",
    "cas<ENTER>cas",
    "cas<TAB>",
    "caas.",
    "tooi,",
    "dd<BS>d",
    "quyeets",
    "giuwax",
    "gioosng",
    "khuyar",
    "hoaf<CLICK>s",
    "AAS",
    "Ddoongf",
    "TIEENGS VIEETJ",
    "z",
    "tiengz",
    "tieengz",
    "baof gioowf",
    "ooo",
    "eee",
    "oong",
    "boong",
    "thoongs",
];

const VNI_EDGE_CASES: &[&str] = &[
    "tie6ng1",
    "d9u7o7ng2",
    "a66",
    "a11",
    "u7o7",
    "uo7",
    "thu7o7ng2",
    "Vie65t",
    "D9",
    "d99",
    "hoa2",
    "khuye6n1",
    "tie6ng 1",
    "a8",
    "a88",
    "q1",
    "123",
    "a1a1",
];

const MACROS: &str = "macros=btw:by the way|vn:Việt Nam|ko:không|đc:được|hn:Hà Nội|t:tôi";

const FEATURE_OPTIONS: &[&str] = &[
    "quick=1",
    "qstart=1",
    "qend=1",
    "qstart=1,qend=1",
    "upper=1",
    "zfwj=1",
    "macro=1",
    "macro=1,autocaps=1",
    "lang=0",
    "lang=0,macro=1,macroen=1",
    "lang=0,macro=1,macroen=1,autocaps=1",
    "quick=1,qstart=1,qend=1,upper=1,zfwj=1,macro=1,autocaps=1",
];

const FEATURE_CASES: &[&str] = &[
    "ccos",
    "gga",
    "kko",
    "nnang",
    "qqa",
    "ppo",
    "tto",
    "uua",
    "Ccos",
    "CCOS",
    "fa ",
    "ja ",
    "wa ",
    "fong ",
    "Jaf ",
    "bag ",
    "bah ",
    "bak ",
    "tieg ",
    "BAG ",
    "fag ",
    "bag.",
    "bag,",
    "Hello. xin chaof",
    "abc.<ENTER>xin chaof",
    "a. b. c",
    "xin.  chaof",
    "btw ",
    "Btw ",
    "BTW ",
    "vn ",
    "VN ",
    "Vn ",
    "ko,",
    "ko.",
    "ddc ",
    "DDc ",
    "hn ",
    "t ",
    "T ",
    "btw.",
    "btw<ENTER>",
    "btw<TAB>",
    "btw<BS>w ",
    "xbtw ",
    "btw  btw ",
    "zalo ",
    "wifi ",
    "jav ",
    "dduwowcj. xin",
    "tieengs <BS><BS>x",
    "vieetj, <BS><BS>s",
    "abc <BS> <BS><BS>d",
    "dd<BS><BS>dd",
    "tie<CLICK>s",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa<BS><BS>s",
    "nghieeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeengs",
    "abcdefghijklmnopqrstuvwxyzabcdefghijkl<BS>s",
    "tooi (khoong) bieets",
    "\"chaof\" 'banj'",
    "100 nawm",
    "a1b2",
    "H2O",
    "x2 dd",
    "toi<ESC>s",
    "tieng<ENTER>s",
    "Vieet<BS><BS><BS><BS><BS>vieejt",
];

#[derive(Clone, Copy)]
enum Modifier {
    Circumflex,
    Breve,
    Horn,
    Stroke,
}

fn decompose(value: char) -> (char, Option<Modifier>) {
    match value {
        'â' => ('a', Some(Modifier::Circumflex)),
        'ê' => ('e', Some(Modifier::Circumflex)),
        'ô' => ('o', Some(Modifier::Circumflex)),
        'ă' => ('a', Some(Modifier::Breve)),
        'ơ' => ('o', Some(Modifier::Horn)),
        'ư' => ('u', Some(Modifier::Horn)),
        'đ' => ('d', Some(Modifier::Stroke)),
        other => (other, None),
    }
}

#[derive(Clone, Copy)]
enum Order {
    Inline,
    ToneAfterVowel,
    Deferred,
}

fn modifier_keys(base: char, modifier: Modifier, vni: bool) -> String {
    match (modifier, vni) {
        (Modifier::Circumflex, false) => base.to_string(),
        (Modifier::Breve | Modifier::Horn, false) => "w".into(),
        (Modifier::Stroke, false) => "d".into(),
        (Modifier::Circumflex, true) => "6".into(),
        (Modifier::Breve, true) => "8".into(),
        (Modifier::Horn, true) => "7".into(),
        (Modifier::Stroke, true) => "9".into(),
    }
}

fn tone_key(tone: usize, vni: bool) -> String {
    if tone == 0 {
        return String::new();
    }
    if vni {
        tone.to_string()
    } else {
        ["", "s", "f", "r", "x", "j"][tone].into()
    }
}

fn syllable_keys(syllable: &str, tone: usize, order: Order, vni: bool) -> String {
    let letters: Vec<(char, Option<Modifier>)> = syllable.chars().map(decompose).collect();
    let last_vowel = letters
        .iter()
        .rposition(|(base, _)| matches!(base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'));
    let mut keys = String::new();
    let mut deferred = String::new();
    let mut previous_horn = false;
    for (index, &(base, modifier)) in letters.iter().enumerate() {
        keys.push(base);
        let horn = matches!(modifier, Some(Modifier::Horn));
        if let Some(modifier) = modifier {
            let extra = modifier_keys(base, modifier, vni);
            match order {
                // One horn key turns a deferred `uo` into `ươ`.
                Order::Deferred if horn && previous_horn => {}
                Order::Deferred => deferred.push_str(&extra),
                _ => keys.push_str(&extra),
            }
        }
        previous_horn = horn;
        if matches!(order, Order::ToneAfterVowel) && Some(index) == last_vowel {
            keys.push_str(&tone_key(tone, vni));
        }
    }
    keys.push_str(&deferred);
    if !matches!(order, Order::ToneAfterVowel) {
        keys.push_str(&tone_key(tone, vni));
    }
    keys
}

fn apply_case(keys: &str, index: usize) -> String {
    if index % 13 == 5 {
        keys.to_uppercase()
    } else if index % 7 == 3 {
        let mut chars = keys.chars();
        chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect())
            .unwrap_or_default()
    } else {
        keys.to_owned()
    }
}

fn option_combo(index: usize) -> String {
    let spell = (index / 2) % 2;
    format!(
        "modern={},spell={spell},restore={spell},free={}",
        index % 2,
        (index / 4) % 2
    )
}

fn corpus() -> Vec<(String, String)> {
    let mut cases = Vec::new();
    let mut syllables = Vec::new();
    for onset in ONSETS {
        for rhyme in RHYMES {
            for tone in 0..6 {
                syllables.push((format!("{onset}{rhyme}"), tone));
            }
        }
    }
    for (method, vni) in [("telex", false), ("vni", true)] {
        for (order_index, order) in [Order::Inline, Order::ToneAfterVowel, Order::Deferred]
            .into_iter()
            .enumerate()
        {
            for (chunk_index, chunk) in syllables.chunks(4).enumerate() {
                let index = chunk_index * 3 + order_index;
                let words: Vec<String> = chunk
                    .iter()
                    .enumerate()
                    .map(|(offset, (syllable, tone))| {
                        apply_case(&syllable_keys(syllable, *tone, order, vni), index + offset)
                    })
                    .collect();
                let ending = if index % 5 == 0 { "." } else { " " };
                cases.push((
                    format!("input={method},{}", option_combo(index)),
                    format!("{}{ending}", words.join(" ")),
                ));
            }
        }
    }
    for combo in 0..8 {
        let options = option_combo(combo);
        for word in ENGLISH {
            cases.push((format!("input=telex,{options}"), format!("{word} ")));
        }
        for case in EDGE_CASES {
            cases.push((format!("input=telex,{options}"), (*case).to_owned()));
            cases.push((format!("input=simple1,{options}"), (*case).to_owned()));
            cases.push((format!("input=simple2,{options}"), (*case).to_owned()));
        }
        for case in VNI_EDGE_CASES {
            cases.push((format!("input=vni,{options}"), (*case).to_owned()));
        }
        for feature in FEATURE_OPTIONS {
            for case in FEATURE_CASES.iter().chain(EDGE_CASES) {
                cases.push((
                    format!("input=telex,{options},{feature},{MACROS}"),
                    (*case).to_owned(),
                ));
            }
            for case in VNI_EDGE_CASES {
                cases.push((
                    format!("input=vni,{options},{feature},{MACROS}"),
                    (*case).to_owned(),
                ));
            }
        }
    }
    cases
}

fn parse_config(text: &str) -> (Config, bool, Vec<(String, String)>) {
    let mut config = Config {
        modern_orthography: false,
        check_spelling: false,
        restore_if_wrong_spelling: false,
        ..Config::default()
    };
    let mut vietnamese = true;
    let mut macros = Vec::new();
    for item in text.split(',') {
        let Some((key, value)) = item.split_once('=') else {
            continue;
        };
        let on = value == "1";
        match key {
            "macros" => {
                for pair in value.split('|') {
                    if let Some((name, content)) = pair.split_once(':') {
                        macros.push((name.to_owned(), content.to_owned()));
                    }
                }
            }
            "input" => {
                config.input_type = match value {
                    "vni" => InputType::Vni,
                    "simple1" => InputType::SimpleTelex1,
                    "simple2" => InputType::SimpleTelex2,
                    _ => InputType::Telex,
                };
            }
            "lang" => vietnamese = on,
            "modern" => config.modern_orthography = on,
            "free" => config.free_mark = on,
            "spell" => config.check_spelling = on,
            "restore" => config.restore_if_wrong_spelling = on,
            "quick" => config.quick_telex = on,
            "qstart" => config.quick_start_consonant = on,
            "qend" => config.quick_end_consonant = on,
            "upper" => config.upper_case_first_char = on,
            "zfwj" => config.allow_consonant_zfwj = on,
            "macro" => config.use_macro = on,
            "macroen" => config.use_macro_in_english_mode = on,
            "autocaps" => config.auto_caps_macro = on,
            _ => {}
        }
    }
    (config, vietnamese, macros)
}

enum Stroke {
    Key(KeyInput),
    Click,
}

fn parse_keys(keys: &str) -> Vec<Stroke> {
    let mut strokes = Vec::new();
    let mut rest = keys;
    while let Some(value) = rest.chars().next() {
        let token = [
            ("<BS>", Key::Backspace),
            ("<ENTER>", Key::Enter),
            ("<TAB>", Key::Tab),
            ("<ESC>", Key::Escape),
        ]
        .into_iter()
        .find(|(name, _)| rest.starts_with(name));
        if let Some((name, key)) = token {
            strokes.push(Stroke::Key(KeyInput::new(key)));
            rest = &rest[name.len()..];
        } else if rest.starts_with("<CLICK>") {
            strokes.push(Stroke::Click);
            rest = &rest["<CLICK>".len()..];
        } else {
            if let Some(input) = KeyInput::from_char(value) {
                strokes.push(Stroke::Key(input));
            }
            rest = &rest[value.len_utf8()..];
        }
    }
    strokes
}

fn apply_key(text: &mut String, key: Key, shift: bool) {
    match key {
        Key::Backspace => {
            text.pop();
        }
        Key::Enter => text.push('⏎'),
        Key::Tab => text.push('\t'),
        _ => {
            if let Some(value) = key.to_char(shift) {
                text.push(value);
            }
        }
    }
}

fn simulate(config: &str, keys: &str) -> String {
    let (config, vietnamese, macros) = parse_config(config);
    let mut engine = Engine::new(config);
    engine.set_vietnamese(vietnamese);
    for (name, content) in macros {
        engine.macros_mut().add(&name, &content);
    }
    let mut text = String::new();
    for stroke in parse_keys(keys) {
        let input = match stroke {
            Stroke::Click => {
                engine.handle_mouse_down();
                continue;
            }
            Stroke::Key(input) => input,
        };
        match engine.handle_key(input) {
            Output::Pass => apply_key(&mut text, input.key, input.shift),
            Output::Replace {
                backspaces,
                text: insert,
                then,
            } => {
                for _ in 0..backspaces {
                    text.pop();
                }
                text.push_str(&insert.replace('\n', "⏎"));
                if let Some(key) = then {
                    apply_key(&mut text, key, input.shift);
                }
            }
        }
    }
    text
}

#[test]
fn matches_expected_texts() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/typing-expected.txt");
    let cases = corpus();
    let got: Vec<String> = cases
        .iter()
        .map(|(config, keys)| simulate(config, keys))
        .collect();
    if env::var_os("VNKEY_BLESS").is_some() {
        let mut text = got.join("\n");
        text.push('\n');
        fs::write(&path, text).expect("write expected texts");
        return;
    }

    let expected = fs::read_to_string(&path).expect("read expected texts");
    let expected: Vec<&str> = expected.lines().collect();
    assert_eq!(expected.len(), cases.len(), "expected text count");

    let mismatches: Vec<String> = cases
        .iter()
        .zip(&got)
        .zip(&expected)
        .filter(|((_, got), want)| got != *want)
        .map(|(((config, keys), got), want)| {
            format!("{config}\t{keys:?}\n  expected: {want:?}\n  got:      {got:?}")
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} of {} cases differ:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
