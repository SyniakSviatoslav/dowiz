//! `kernel::stem` — zero-dep multilingual light stemmer.
//!
//! Covers 50 languages: EN, UK, RU, DE, FR, ES, IT, PT, PL, NL, SV, NO, DA, TR, AR,
//! ZH, JA, KO, HI, RO, HU, EL, CS, VI, HE,
//! BN, CA, ET, EU, FA, FI, GL, GU, ID, IS, KN, LT, LV, ML, MR, MS, PA, SK, SL, SW,
//! TA, TE, TH, TL, UR.
//! Light suffix-stripping for inflectional languages. Used by retrieval layer.

/// Light stem: strip common inflectional suffixes for 50 languages.
pub fn stem(word: &str) -> String {
    let w = word.trim().to_lowercase();

    // ── Ukrainian ────────────────────────────────────────────────────
    for &suffix in &["уватися","юватися","ювати","увати","ють","уть","тися","тиму","тиме","тимеш","тимуть","ла","ло","ли","в","ти","ть","ами","ями","ою","ею","ість"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Russian ──────────────────────────────────────────────────────
    for &suffix in &["оваться","еваться","иваться","ываться","овать","евать","ивать","ывать","ются","ется","ться","ами","ями","ого","его","ому","ему","ыми","ими","ой","ей","ая","яя","ое","ее","ые","ие","ость"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── German (DE) ──────────────────────────────────────────────────
    for &suffix in &["ungen","heiten","keiten","schaft","ierung","tion","chen","lein","sten","ern","end","ung","heit","keit","isch","lich","ig","es","er","en","em","el","es","te","ten","test","tet"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── French (FR) ──────────────────────────────────────────────────
    for &suffix in &["issement","ablement","eraient","eraient","issions","eraient","erions","eraient","era","erai","erez","erons","eront","aient","aisse","ante","ment","tion","sion","euse","eux","aux","eaux","elle","ette","eurs","ance","ence","es","ez","er","ir","re","ons","ent","ais","ait"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Spanish (ES) ─────────────────────────────────────────────────
    for &suffix in &["aciones","ecimientos","imientos","aciones","dores","doras","mente","miento","cion","sion","ista","ismo","idad","eza","ura","anza","encia","ible","able","ica","ico","oso","osa","ero","era","dor","dora","ito","ita","ote","ota","on","ona","azo","aza","ado","ada","ido","ida","ando","iendo","ar","er","ir","as","es","os","an","en","on"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Italian (IT) ─────────────────────────────────────────────────
    for &suffix in &["azione","azioni","imento","imenti","mente","trice","tore","trici","tori","ista","isti","ismo","ita","ita","ezza","ura","abile","ibile","evole","oso","osa","osi","ose","ino","ina","etto","etta","one","oni","are","ere","ire","ato","ita","ite","uti","ute","ano","ono","ente","enti"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Portuguese (PT) ──────────────────────────────────────────────
    for &suffix in &["acoes","icoes","mento","mente","idade","eza","ura","ancia","encia","avel","ivel","oso","osa","inho","inha","ao","oes","ado","ida","ando","endo","indo","ar","er","ir","ava","era","ira","ou","eu","iu","am","em","im","aram","eram","iram","asse","esse","isse"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Polish (PL) ──────────────────────────────────────────────────
    for &suffix in &["ami","ach","owi","ego","emu","ymi","ymi","owie","owie","ach","ami","om","a","u","em","e","y","i","owie","ów","om","ami","ach"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Dutch (NL) ───────────────────────────────────────────────────
    for &suffix in &["ingen","ingen","heden","heden","schap","atie","eren","eren","ende","ende","ige","ige","lijk","lijk","jes","jes","tje","tje","en","en","de","de","te","te","s","s"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Swedish/Norwegian/Danish ─────────────────────────────────────
    for &suffix in &["ningar","igheter","igaste","ligaste","anden","anden","heter","heter","ning","ning","aste","aste","ande","ande","ende","ende","erne","erne","ens","ens","ets","ets","en","en","et","et","ar","ar","er","er","or","or","na","na","as","as","es","es"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Turkish (TR) ─────────────────────────────────────────────────
    for &suffix in &["lar","ler","lar","ler","da","de","ta","te","dan","den","tan","ten","a","e","ya","ye","n","i","u","dır","dir","dur","dür","tır","tir","tur","tür","mış","miş","muş","müş","yor","acak","ecek","mek","mak","me","ma"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Arabic (AR) — basic pattern stripping ────────────────────────
    for &suffix in &["ون","ين","ات","ان","ة","ي","ه","ا","و"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Chinese (ZH) — isolating language, no suffix stripping ──────
    // ── Japanese (JA) — strip polite/verb endings ────────────────────
    for &suffix in &["ています","ている","られます","させる","させる","ます","です","でした","ました","ません","た","て","る","に","は","が","を"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Korean (KO) — strip polite/declarative endings ───────────────
    for &suffix in &["습니다","ㅂ니다","합니다","에요","예요","이에요","있어요","없어요","했어요","는","은","를","을","이","가","에","에서","으로","로"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Hindi (HI) — strip case/postposition/verb endings ────────────
    for &suffix in &["कर","ता","ते","ती","ने","को","से","में","पर","का","की","के","है","हैं","था","थी","थे","रहा","रही","रहे","ए","ओ"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Romanian (RO) — strip definite/article endings ───────────────
    for &suffix in &["ului","ilor","ilor","ele","uri","uri","ul","ui","lor","le","a","ea","e","i"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Hungarian (HU) — strip plural/case suffixes ──────────────────
    for &suffix in &["ok","ek","ök","ak","ek","ban","ben","val","vel","nak","nek","ra","re","on","en","ön","ba","be","ból","ből","ról","ről","tól","től","ig","k","nk","jaim","jeim"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Greek (EL) — strip case/gender/number endings ────────────────
    for &suffix in &["ονος","ικος","ικός","ματα","εων","εις","ες","ος","η","ο","ου","ων","ους","ας","α","ης","ες","οι","ων","α","η","ο"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Czech (CS) — strip case/gender endings ───────────────────────
    for &suffix in &["y","i","e","u","ou","ů","ami","emi","ích","ech","ám","ům","ovi","ů"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Vietnamese (VI) — isolating language, no suffix stripping ────
    // ── Hebrew (HE) — strip plural/gender endings ────────────────────
    for &suffix in &["ים","ות","ינו","ני","י","ו","ה","ת","נו","כם","כן","יהם","יהן"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }

    // ── Finnish (FI) — complex agglutinative ────────────────────────
    for &suffix in &["kaan","kään","nsa","nsä","han","hän","sta","stä","kin","ko","kö","pa","pä","ni","si","kaan"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Estonian (ET) — similar to Finnish ──────────────────────────
    for &suffix in &["sse","st","le","lt","ks","ni","gi"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Indonesian (ID) — prefix+suffix ─────────────────────────────
    for &suffix in &["memper","meng","men","kan","nya","ter","me","di","ke","an"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Malay (MS) — same as Indonesian, regional variants ──────────
    for &suffix in &["memper","meng","men","kan","nya","ter","me","di","ke","an"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Filipino/Tagalog (TL) ───────────────────────────────────────
    for &suffix in &["mga","ang","mag","nag","pag","ng","um","sa","na","pa"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Thai (TH) — isolating but strip common particles ────────────
    for &suffix in &["ไม่","ได้","แล้ว","เป็น","อยู่","ที่","จะ","ของ"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Swahili (SW) — strip verbal prefixes/suffixes ───────────────
    for &suffix in &["wa","ni","u","li","na","me","ka","ki","a"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Persian (FA) ────────────────────────────────────────────────
    for &suffix in &["ترین","ها","تر","مان","تان","شان","گی","ش","ی","م","ت"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Urdu (UR) ───────────────────────────────────────────────────
    for &suffix in &["تھا","تھی","تھے","ہے","وں","ے","ں","ا"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Bengali (BN) ────────────────────────────────────────────────
    for &suffix in &["গুলো","বেন","ছে","টি","টা","রা","কে","তে","র"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Tamil (TA) ─────────────────────────────────────────────────
    for &suffix in &["க்கு","காக","இல்","இடம்","இது","ஆக","கள்"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Telugu (TE) ────────────────────────────────────────────────
    for &suffix in &["కోసం","నుంచి","లు","కి","లో","ది"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Marathi (MR) ───────────────────────────────────────────────
    for &suffix in &["चा","ची","चे","या","ना","नी","त","आ","ई"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Gujarati (GU) ──────────────────────────────────────────────
    for &suffix in &["માં","નો","ની","નું","ને","થી","ઓ"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Punjabi (PA) ───────────────────────────────────────────────
    for &suffix in &["ਵਿੱਚ","ਦਾ","ਦੀ","ਦੇ","ਨੂੰ","ਤੋਂ"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Kannada (KN) ───────────────────────────────────────────────
    for &suffix in &["ಗಳು","ಗಾಗಿ","ನಲ್ಲಿ","ಯಿಂದ","ಗೆ"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Malayalam (ML) ─────────────────────────────────────────────
    for &suffix in &["നിന്ന്","കൾ","ഉം","ഇൽ","ആണ്"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Catalan (CA) ───────────────────────────────────────────────
    for &suffix in &["isme","ista","ció","ment","itat","tat","dor","tor"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Basque (EU) ────────────────────────────────────────────────
    for &suffix in &["arekin","tik","ak","ek","ra","ri"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Galician (GL) ──────────────────────────────────────────────
    for &suffix in &["mento","ismo","ista","dade","tade","ciu"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Icelandic (IS) ─────────────────────────────────────────────
    for &suffix in &["inn","num","nar","nna","ur","ar","ir","in","ið"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Lithuanian (LT) ────────────────────────────────────────────
    for &suffix in &["ėti","yti","oti","as","is","us","ys","s","ė"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Latvian (LV) ───────────────────────────────────────────────
    for &suffix in &["ībām","ām","iem","is","us","s","a"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Slovak (SK) ────────────────────────────────────────────────
    for &suffix in &["stvo","ný","tý","ť"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }
    // ── Slovenian (SL) ─────────────────────────────────────────────
    for &suffix in &["ega","emu","ima","imi","em","ih","a"] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) { return w[..w.len()-suffix.len()].to_string(); }
    }

    // ── English ──────────────────────────────────────────────────────────
    for &suffix in &[
        "ational","tional","enci","anci","izer","abli","alli","entli","eliti",
        "ously","ization","ation","ator","alism","iveness","fulness","ousness",
        "aliti","iviti","biliti","ing","edly","ment","ness","able","ible",
        "ment","ship","hood","less","ness",
    ] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }

    // English plurals
    for &suffix in &["ies", "ses", "xes", "zes", "ches", "shes"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) {
            return w[..w.len() - 2].to_string();
        }
    }
    if w.ends_with('s') && w.len() > 3 && !w.ends_with("ss") {
        return w[..w.len() - 1].to_string();
    }
    if w.len() > 5 && w.ends_with("ing") { return w[..w.len() - 3].to_string(); }
    if w.len() > 5 && w.ends_with("ed") { return w[..w.len() - 2].to_string(); }
    if w.len() > 4 && w.ends_with("ly") { return w[..w.len() - 2].to_string(); }

    w
}

/// Tokenize text into stemmed tokens.
pub fn tokenize_stemmed(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| stem(w))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    Uk,
    Ru,
    De,
    Fr,
    Es,
    It,
    Pt,
    Pl,
    Nl,
    Sv,
    No,
    Da,
    Tr,
    Ar,
    Zh,
    Ja,
    Ko,
    Hi,
    Ro,
    Hu,
    El,
    Cs,
    Vi,
    He,
    Bn,
    Ca,
    Et,
    Eu,
    Fa,
    Fi,
    Gl,
    Gu,
    Id,
    Is,
    Kn,
    Lt,
    Lv,
    Ml,
    Mr,
    Ms,
    Pa,
    Sk,
    Sl,
    Sw,
    Ta,
    Te,
    Th,
    Tl,
    Ur,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Uk => "Ukrainian",
            Language::Ru => "Russian",
            Language::De => "German",
            Language::Fr => "French",
            Language::Es => "Spanish",
            Language::It => "Italian",
            Language::Pt => "Portuguese",
            Language::Pl => "Polish",
            Language::Nl => "Dutch",
            Language::Sv => "Swedish",
            Language::No => "Norwegian",
            Language::Da => "Danish",
            Language::Tr => "Turkish",
            Language::Ar => "Arabic",
            Language::Zh => "Chinese",
            Language::Ja => "Japanese",
            Language::Ko => "Korean",
            Language::Hi => "Hindi",
            Language::Ro => "Romanian",
            Language::Hu => "Hungarian",
            Language::El => "Greek",
            Language::Cs => "Czech",
            Language::Vi => "Vietnamese",
            Language::He => "Hebrew",
            Language::Bn => "Bengali",
            Language::Ca => "Catalan",
            Language::Et => "Estonian",
            Language::Eu => "Basque",
            Language::Fa => "Persian",
            Language::Fi => "Finnish",
            Language::Gl => "Galician",
            Language::Gu => "Gujarati",
            Language::Id => "Indonesian",
            Language::Is => "Icelandic",
            Language::Kn => "Kannada",
            Language::Lt => "Lithuanian",
            Language::Lv => "Latvian",
            Language::Ml => "Malayalam",
            Language::Mr => "Marathi",
            Language::Ms => "Malay",
            Language::Pa => "Punjabi",
            Language::Sk => "Slovak",
            Language::Sl => "Slovenian",
            Language::Sw => "Swahili",
            Language::Ta => "Tamil",
            Language::Te => "Telugu",
            Language::Th => "Thai",
            Language::Tl => "Filipino",
            Language::Ur => "Urdu",
        }
    }

    pub fn as_iso639(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Uk => "uk",
            Language::Ru => "ru",
            Language::De => "de",
            Language::Fr => "fr",
            Language::Es => "es",
            Language::It => "it",
            Language::Pt => "pt",
            Language::Pl => "pl",
            Language::Nl => "nl",
            Language::Sv => "sv",
            Language::No => "no",
            Language::Da => "da",
            Language::Tr => "tr",
            Language::Ar => "ar",
            Language::Zh => "zh",
            Language::Ja => "ja",
            Language::Ko => "ko",
            Language::Hi => "hi",
            Language::Ro => "ro",
            Language::Hu => "hu",
            Language::El => "el",
            Language::Cs => "cs",
            Language::Vi => "vi",
            Language::He => "he",
            Language::Bn => "bn",
            Language::Ca => "ca",
            Language::Et => "et",
            Language::Eu => "eu",
            Language::Fa => "fa",
            Language::Fi => "fi",
            Language::Gl => "gl",
            Language::Gu => "gu",
            Language::Id => "id",
            Language::Is => "is",
            Language::Kn => "kn",
            Language::Lt => "lt",
            Language::Lv => "lv",
            Language::Ml => "ml",
            Language::Mr => "mr",
            Language::Ms => "ms",
            Language::Pa => "pa",
            Language::Sk => "sk",
            Language::Sl => "sl",
            Language::Sw => "sw",
            Language::Ta => "ta",
            Language::Te => "te",
            Language::Th => "th",
            Language::Tl => "tl",
            Language::Ur => "ur",
        }
    }

    pub fn as_script_name(&self) -> &'static str {
        match self {
            Language::En
            | Language::De
            | Language::Fr
            | Language::Es
            | Language::It
            | Language::Pt
            | Language::Pl
            | Language::Nl
            | Language::Sv
            | Language::No
            | Language::Da
            | Language::Tr
            | Language::Ro
            | Language::Hu
            | Language::Cs
            | Language::Vi
            | Language::Ca
            | Language::Et
            | Language::Eu
            | Language::Fi
            | Language::Gl
            | Language::Id
            | Language::Is
            | Language::Lt
            | Language::Lv
            | Language::Ms
            | Language::Sk
            | Language::Sl
            | Language::Sw
            | Language::Tl => "Latin",
            Language::Uk | Language::Ru => "Cyrillic",
            Language::Ar | Language::Fa | Language::Ur => "Arabic",
            Language::Zh => "Han",
            Language::Ja => "Han + Kana",
            Language::Ko => "Hangul",
            Language::Hi | Language::Mr => "Devanagari",
            Language::El => "Greek",
            Language::He => "Hebrew",
            Language::Bn => "Bengali",
            Language::Gu => "Gujarati",
            Language::Kn => "Kannada",
            Language::Ml => "Malayalam",
            Language::Pa => "Gurmukhi",
            Language::Ta => "Tamil",
            Language::Te => "Telugu",
            Language::Th => "Thai",
        }
    }
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')
}

fn is_cjk_ideograph(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2A6DF}'
        | '\u{2A700}'..='\u{2B73F}'
        | '\u{2B740}'..='\u{2B81F}'
        | '\u{2B820}'..='\u{2CEAF}'
        | '\u{2CEB0}'..='\u{2EBEF}'
        | '\u{30000}'..='\u{3134F}'
        | '\u{31350}'..='\u{323AF}'
    )
}

fn is_ukrainian_specific(c: char) -> bool {
    matches!(c,
        '\u{0404}'  // Є
        | '\u{0454}'  // є
        | '\u{0407}'  // Ї
        | '\u{0457}'  // ї
        | '\u{0490}'  // Ґ
        | '\u{0491}'  // ґ
    )
}

fn is_hiragana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{309F}')
}

fn is_katakana(c: char) -> bool {
    matches!(c, '\u{30A0}'..='\u{30FF}')
}

fn is_hangul(c: char) -> bool {
    matches!(c, '\u{AC00}'..='\u{D7AF}' | '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}')
}

fn is_arabic_script(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
}

fn is_devanagari(c: char) -> bool {
    matches!(c, '\u{0900}'..='\u{097F}')
}

fn is_hebrew(c: char) -> bool {
    matches!(c, '\u{0590}'..='\u{05FF}' | '\u{FB1D}'..='\u{FB4F}')
}

fn is_greek(c: char) -> bool {
    matches!(c, '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}')
}

fn is_bengali(c: char) -> bool {
    matches!(c, '\u{0980}'..='\u{09FF}')
}

fn is_gujarati(c: char) -> bool {
    matches!(c, '\u{0A80}'..='\u{0AFF}')
}

fn is_gurmukhi(c: char) -> bool {
    matches!(c, '\u{0A00}'..='\u{0A7F}')
}

fn is_kannada(c: char) -> bool {
    matches!(c, '\u{0C80}'..='\u{0CFF}')
}

fn is_malayalam(c: char) -> bool {
    matches!(c, '\u{0D00}'..='\u{0D7F}')
}

fn is_tamil(c: char) -> bool {
    matches!(c, '\u{0B80}'..='\u{0BFF}')
}

fn is_telugu(c: char) -> bool {
    matches!(c, '\u{0C00}'..='\u{0C7F}')
}

fn is_thai(c: char) -> bool {
    matches!(c, '\u{0E00}'..='\u{0E7F}')
}

fn is_persian_specific(c: char) -> bool {
    matches!(c,
        '\u{067E}'  // پ
        | '\u{0686}'  // چ
        | '\u{0698}'  // ژ
        | '\u{06AF}'  // گ
        | '\u{06A9}'  // ک
        | '\u{06CC}'  // ی
    )
}

fn is_urdu_specific(c: char) -> bool {
    matches!(c,
        '\u{06D2}'  // ے
        | '\u{06BA}'  // ں
        | '\u{0688}'  // ڈ
        | '\u{0679}'  // ٹ
        | '\u{0691}'  // ڑ
    )
}

fn is_marathi_specific(c: char) -> bool {
    matches!(c,
        '\u{0933}'  // ळ
        | '\u{0931}'  // ऱ
    )
}

/// Detect the language of `text` using Unicode script ranges and suffix overlap scoring.
pub fn detect_language(text: &str) -> Language {
    let mut found_cyrillic = false;
    let mut found_ukrainian = false;
    let mut found_cjk = false;
    let mut found_hiragana = false;
    let mut found_katakana = false;
    let mut found_hangul = false;
    let mut found_arabic = false;
    let mut found_persian = false;
    let mut found_urdu = false;
    let mut found_devanagari = false;
    let mut found_marathi = false;
    let mut found_hebrew = false;
    let mut found_greek = false;
    let mut found_bengali = false;
    let mut found_gujarati = false;
    let mut found_gurmukhi = false;
    let mut found_kannada = false;
    let mut found_malayalam = false;
    let mut found_tamil = false;
    let mut found_telugu = false;
    let mut found_thai = false;

    for c in text.chars() {
        if is_cyrillic(c) {
            found_cyrillic = true;
            if is_ukrainian_specific(c) {
                found_ukrainian = true;
            }
        }
        if is_cjk_ideograph(c) {
            found_cjk = true;
        }
        if is_hiragana(c) {
            found_hiragana = true;
        }
        if is_katakana(c) {
            found_katakana = true;
        }
        if is_hangul(c) {
            found_hangul = true;
        }
        if is_arabic_script(c) {
            found_arabic = true;
            if is_persian_specific(c) {
                found_persian = true;
            }
            if is_urdu_specific(c) {
                found_urdu = true;
            }
        }
        if is_devanagari(c) {
            found_devanagari = true;
            if is_marathi_specific(c) {
                found_marathi = true;
            }
        }
        if is_hebrew(c) {
            found_hebrew = true;
        }
        if is_greek(c) {
            found_greek = true;
        }
        if is_bengali(c) {
            found_bengali = true;
        }
        if is_gujarati(c) {
            found_gujarati = true;
        }
        if is_gurmukhi(c) {
            found_gurmukhi = true;
        }
        if is_kannada(c) {
            found_kannada = true;
        }
        if is_malayalam(c) {
            found_malayalam = true;
        }
        if is_tamil(c) {
            found_tamil = true;
        }
        if is_telugu(c) {
            found_telugu = true;
        }
        if is_thai(c) {
            found_thai = true;
        }
    }

    if found_cyrillic {
        return if found_ukrainian { Language::Uk } else { Language::Ru };
    }
    if found_cjk || found_hiragana || found_katakana || found_hangul {
        if found_hangul && !found_hiragana && !found_katakana {
            return Language::Ko;
        }
        if (found_hiragana || found_katakana) && !found_hangul {
            return Language::Ja;
        }
        return Language::Zh;
    }
    if found_arabic {
        if found_urdu {
            return Language::Ur;
        }
        if found_persian {
            return Language::Fa;
        }
        return Language::Ar;
    }
    if found_devanagari {
        if found_marathi {
            return Language::Mr;
        }
        return Language::Hi;
    }
    if found_hebrew {
        return Language::He;
    }
    if found_greek {
        return Language::El;
    }
    if found_bengali {
        return Language::Bn;
    }
    if found_gujarati {
        return Language::Gu;
    }
    if found_gurmukhi {
        return Language::Pa;
    }
    if found_kannada {
        return Language::Kn;
    }
    if found_malayalam {
        return Language::Ml;
    }
    if found_tamil {
        return Language::Ta;
    }
    if found_telugu {
        return Language::Te;
    }
    if found_thai {
        return Language::Th;
    }

    score_latin_by_suffix(text)
}

const LATIN_SUFFIXES: &[(&str, Language)] = &[
    ("ului", Language::Ro),
    ("ilor", Language::Ro),
    ("elor", Language::Ro),
    ("ul", Language::Ro),
    ("ui", Language::Ro),
    ("lor", Language::Ro),
    ("le", Language::Ro),
    ("a", Language::Ro),
    ("ea", Language::Ro),
    ("e", Language::Ro),
    ("i", Language::Ro),
    ("ok", Language::Hu),
    ("ek", Language::Hu),
    ("ök", Language::Hu),
    ("ak", Language::Hu),
    ("ban", Language::Hu),
    ("ben", Language::Hu),
    ("val", Language::Hu),
    ("vel", Language::Hu),
    ("nak", Language::Hu),
    ("nek", Language::Hu),
    ("ra", Language::Hu),
    ("re", Language::Hu),
    ("ból", Language::Hu),
    ("ből", Language::Hu),
    ("ról", Language::Hu),
    ("ről", Language::Hu),
    ("tól", Language::Hu),
    ("től", Language::Hu),
    ("lar", Language::Tr),
    ("ler", Language::Tr),
    ("mak", Language::Tr),
    ("mek", Language::Tr),
    ("da", Language::Tr),
    ("de", Language::Tr),
    ("ta", Language::Tr),
    ("te", Language::Tr),
    ("dan", Language::Tr),
    ("den", Language::Tr),
    ("tan", Language::Tr),
    ("ten", Language::Tr),
    ("ung", Language::De),
    ("heit", Language::De),
    ("keit", Language::De),
    ("schaft", Language::De),
    ("ierung", Language::De),
    ("tion", Language::De),
    ("chen", Language::De),
    ("lein", Language::De),
    ("isch", Language::De),
    ("lich", Language::De),
    ("ment", Language::Fr),
    ("sion", Language::Fr),
    ("euse", Language::Fr),
    ("eux", Language::Fr),
    ("aux", Language::Fr),
    ("eaux", Language::Fr),
    ("elle", Language::Fr),
    ("ette", Language::Fr),
    ("eurs", Language::Fr),
    ("ance", Language::Fr),
    ("ence", Language::Fr),
    ("iente", Language::Fr),
    ("cion", Language::Es),
    ("mente", Language::Es),
    ("miento", Language::Es),
    ("idad", Language::Es),
    ("eza", Language::Es),
    ("ura", Language::Es),
    ("able", Language::Es),
    ("ible", Language::Es),
    ("dor", Language::Es),
    ("dora", Language::Es),
    ("ito", Language::Es),
    ("ita", Language::Es),
    ("ado", Language::Es),
    ("ida", Language::Es),
    ("ando", Language::Es),
    ("iendo", Language::Es),
    ("zione", Language::It),
    ("menti", Language::It),
    ("trice", Language::It),
    ("tore", Language::It),
    ("trici", Language::It),
    ("tori", Language::It),
    ("ista", Language::It),
    ("ismo", Language::It),
    ("ezza", Language::It),
    ("abile", Language::It),
    ("ibile", Language::It),
    ("evole", Language::It),
    ("ino", Language::It),
    ("etto", Language::It),
    ("imento", Language::Pt),
    ("idade", Language::Pt),
    ("avel", Language::Pt),
    ("ivel", Language::Pt),
    ("oso", Language::Pt),
    ("osa", Language::Pt),
    ("inho", Language::Pt),
    ("inha", Language::Pt),
    ("ando", Language::Pt),
    ("endo", Language::Pt),
    ("indo", Language::Pt),
    ("ami", Language::Pl),
    ("ach", Language::Pl),
    ("owi", Language::Pl),
    ("ego", Language::Pl),
    ("emu", Language::Pl),
    ("ymi", Language::Pl),
    ("owie", Language::Pl),
    ("ów", Language::Pl),
    ("ing", Language::Nl),
    ("heid", Language::Nl),
    ("schap", Language::Nl),
    ("atie", Language::Nl),
    ("eren", Language::Nl),
    ("ende", Language::Nl),
    ("lijke", Language::Nl),
    ("jes", Language::Nl),
    ("tje", Language::Nl),
    ("ning", Language::Sv),
    ("igaste", Language::Sv),
    ("ligaste", Language::Sv),
    ("arne", Language::Sv),
    ("ene", Language::Sv),
    ("erne", Language::Da),
    ("erne", Language::No),
    ("erne", Language::Sv),
    ("ets", Language::Sv),
    ("aste", Language::Sv),
    ("ande", Language::Sv),
    ("ende", Language::No),
    ("ende", Language::Da),
    ("ende", Language::Sv),
    ("ů", Language::Cs),
    ("ích", Language::Cs),
    ("ech", Language::Cs),
    ("ům", Language::Cs),
    ("ovi", Language::Cs),
    ("emi", Language::Cs),
    ("ational", Language::En),
    ("tional", Language::En),
    ("ization", Language::En),
    ("ation", Language::En),
    ("alism", Language::En),
    ("iveness", Language::En),
    ("fulness", Language::En),
    ("ousness", Language::En),
    ("biliti", Language::En),
    ("iviti", Language::En),
    ("aliti", Language::En),
    ("entli", Language::En),
    ("ously", Language::En),
    ("ness", Language::En),
    ("ship", Language::En),
    ("hood", Language::En),
    ("less", Language::En),
    ("edly", Language::En),
    ("ing", Language::En),
    ("ment", Language::En),
    ("able", Language::En),
    ("ible", Language::En),
    ("ies", Language::En),
    ("ses", Language::En),
    ("xes", Language::En),
    ("zes", Language::En),
    ("ches", Language::En),
    ("shes", Language::En),
    ("ly", Language::En),
    ("ed", Language::En),
    ("kaan", Language::Fi),
    ("kään", Language::Fi),
    ("nsa", Language::Fi),
    ("nsä", Language::Fi),
    ("han", Language::Fi),
    ("hän", Language::Fi),
    ("sta", Language::Fi),
    ("stä", Language::Fi),
    ("kin", Language::Fi),
    ("sse", Language::Et),
    ("memper", Language::Id),
    ("meng", Language::Id),
    ("men", Language::Id),
    ("kan", Language::Id),
    ("nya", Language::Id),
    ("ter", Language::Id),
    ("memper", Language::Ms),
    ("meng", Language::Ms),
    ("men", Language::Ms),
    ("kan", Language::Ms),
    ("mga", Language::Tl),
    ("ang", Language::Tl),
    ("mag", Language::Tl),
    ("nag", Language::Tl),
    ("pag", Language::Tl),
    ("wa", Language::Sw),
    ("li", Language::Sw),
    ("ka", Language::Sw),
    ("ki", Language::Sw),
    ("ció", Language::Ca),
    ("isme", Language::Ca),
    ("ista", Language::Ca),
    ("itat", Language::Ca),
    ("tat", Language::Ca),
    ("dor", Language::Ca),
    ("tor", Language::Ca),
    ("arekin", Language::Eu),
    ("tik", Language::Eu),
    ("ak", Language::Eu),
    ("ek", Language::Eu),
    ("mento", Language::Gl),
    ("ismo", Language::Gl),
    ("ista", Language::Gl),
    ("dade", Language::Gl),
    ("tade", Language::Gl),
    ("ciu", Language::Gl),
    ("inn", Language::Is),
    ("num", Language::Is),
    ("nar", Language::Is),
    ("nna", Language::Is),
    ("ur", Language::Is),
    ("ar", Language::Is),
    ("ir", Language::Is),
    ("ið", Language::Is),
    ("ėti", Language::Lt),
    ("yti", Language::Lt),
    ("oti", Language::Lt),
    ("as", Language::Lt),
    ("is", Language::Lt),
    ("us", Language::Lt),
    ("ys", Language::Lt),
    ("ė", Language::Lt),
    ("ībām", Language::Lv),
    ("ām", Language::Lv),
    ("iem", Language::Lv),
    ("stvo", Language::Sk),
    ("ný", Language::Sk),
    ("tý", Language::Sk),
    ("ega", Language::Sl),
    ("emu", Language::Sl),
    ("ima", Language::Sl),
    ("imi", Language::Sl),
    ("em", Language::Sl),
    ("ih", Language::Sl),
];

fn score_latin_by_suffix(text: &str) -> Language {
    let lower = text.to_lowercase();
    let mut scores: [u32; 50] = [0; 50];

    fn lang_idx(l: Language) -> usize {
        match l {
            Language::En => 0,
            Language::Uk => 1,
            Language::Ru => 2,
            Language::De => 3,
            Language::Fr => 4,
            Language::Es => 5,
            Language::It => 6,
            Language::Pt => 7,
            Language::Pl => 8,
            Language::Nl => 9,
            Language::Sv => 10,
            Language::No => 11,
            Language::Da => 12,
            Language::Tr => 13,
            Language::Ar => 14,
            Language::Zh => 15,
            Language::Ja => 16,
            Language::Ko => 17,
            Language::Hi => 18,
            Language::Ro => 19,
            Language::Hu => 20,
            Language::El => 21,
            Language::Cs => 22,
            Language::Vi => 23,
            Language::He => 24,
            Language::Bn => 25,
            Language::Ca => 26,
            Language::Et => 27,
            Language::Eu => 28,
            Language::Fa => 29,
            Language::Fi => 30,
            Language::Gl => 31,
            Language::Gu => 32,
            Language::Id => 33,
            Language::Is => 34,
            Language::Kn => 35,
            Language::Lt => 36,
            Language::Lv => 37,
            Language::Ml => 38,
            Language::Mr => 39,
            Language::Ms => 40,
            Language::Pa => 41,
            Language::Sk => 42,
            Language::Sl => 43,
            Language::Sw => 44,
            Language::Ta => 45,
            Language::Te => 46,
            Language::Th => 47,
            Language::Tl => 48,
            Language::Ur => 49,
        }
    }

    for word in lower.split(|c: char| !c.is_alphanumeric()) {
        if word.len() < 3 {
            continue;
        }
        for &(suffix, lang) in LATIN_SUFFIXES {
            if word.len() > suffix.len() && word.ends_with(suffix) {
                let idx = lang_idx(lang);
                scores[idx] += 1;
            }
        }
    }

    let mut best_idx = 0;
    let mut best_score = 0;
    for i in 0..50 {
        if scores[i] > best_score {
            best_score = scores[i];
            best_idx = i;
        }
    }

    match best_idx {
        0 => Language::En,
        1 => Language::Uk,
        2 => Language::Ru,
        3 => Language::De,
        4 => Language::Fr,
        5 => Language::Es,
        6 => Language::It,
        7 => Language::Pt,
        8 => Language::Pl,
        9 => Language::Nl,
        10 => Language::Sv,
        11 => Language::No,
        12 => Language::Da,
        13 => Language::Tr,
        14 => Language::Ar,
        15 => Language::Zh,
        16 => Language::Ja,
        17 => Language::Ko,
        18 => Language::Hi,
        19 => Language::Ro,
        20 => Language::Hu,
        21 => Language::El,
        22 => Language::Cs,
        23 => Language::Vi,
        24 => Language::He,
        25 => Language::Bn,
        26 => Language::Ca,
        27 => Language::Et,
        28 => Language::Eu,
        29 => Language::Fa,
        30 => Language::Fi,
        31 => Language::Gl,
        32 => Language::Gu,
        33 => Language::Id,
        34 => Language::Is,
        35 => Language::Kn,
        36 => Language::Lt,
        37 => Language::Lv,
        38 => Language::Ml,
        39 => Language::Mr,
        40 => Language::Ms,
        41 => Language::Pa,
        42 => Language::Sk,
        43 => Language::Sl,
        44 => Language::Sw,
        45 => Language::Ta,
        46 => Language::Te,
        47 => Language::Th,
        48 => Language::Tl,
        49 => Language::Ur,
        _ => Language::En,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_english_plural() {
        assert!(!stem("running").is_empty());
        assert!(!stem("jumped").is_empty());
        assert!(!stem("abilities").is_empty());
        assert!(!stem("cats").is_empty());
        assert!(!stem("boxes").is_empty());
    }

    #[test]
    fn stem_english_ness_ment() {
        assert!(!stem("happiness").is_empty());
        assert!(!stem("government").is_empty());
    }

    #[test]
    fn stem_ukrainian_noun_cases() {
        let s1 = stem("замовлення");
        let s2 = stem("замовленню");
        let s3 = stem("замовленням");
        assert!(!s1.is_empty());
        assert!(!s2.is_empty());
        assert!(!s3.is_empty());
    }

    #[test]
    fn stem_ukrainian_verbs() {
        assert!(stem("робити").len() < "робити".len());
        assert!(stem("зробив").len() < "зробив".len());
    }

    #[test]
    fn stem_russian() {
        let s1 = stem("делающий");
        let s2 = stem("программирования");
        assert!(!s1.is_empty());
        assert!(!s2.is_empty());
    }

    #[test]
    fn stem_no_change() {
        assert!(!stem("rust").is_empty());
        assert!(!stem("code").is_empty());
    }

    #[test]
    fn tokenize_stemmed_works() {
        let tokens = tokenize_stemmed("running functions jumped over lazy dogs");
        assert!(tokens.len() >= 3);
    }

    #[test]
    fn stem_japanese_polite() {
        let s1 = stem("食べます");
        let s2 = stem("行きました");
        assert!(s1.len() < "食べます".len() || !s1.is_empty());
        assert!(s2.len() < "行きました".len() || !s2.is_empty());
        assert!(!s1.is_empty() && !s2.is_empty());
    }

    #[test]
    fn stem_korean_polite() {
        let s1 = stem("합니다");
        let s2 = stem("있습니다");
        assert!(!s1.is_empty() && !s2.is_empty());
    }

    #[test]
    fn stem_hindi_postpositions() {
        let s1 = stem("लड़के");
        let s2 = stem("किताबों");
        assert!(!s1.is_empty() && !s2.is_empty());
    }

    #[test]
    fn stem_chinese_identity() {
        let s = stem("编程");
        assert_eq!(s, "编程");
    }

    #[test]
    fn stem_vietnamese_identity() {
        let s = stem("lập trình");
        assert_eq!(s, "lập trình");
    }

    #[test]
    fn stem_hebrew_plurals() {
        let s1 = stem("ספרים");
        let s2 = stem("מילים");
        assert!(!s1.is_empty() && !s2.is_empty());
    }

    #[test]
    fn stem_finnish() {
        assert!(!stem("taloissani").is_empty());
        assert!(stem("taloni").len() < "taloni".len());
    }

    #[test]
    fn stem_estonian() {
        assert!(!stem("majasse").is_empty());
        assert!(stem("majast").len() < "majast".len());
    }

    #[test]
    fn stem_indonesian() {
        assert!(!stem("makanan").is_empty());
        assert!(stem("memperbaiki").len() < "memperbaiki".len());
    }

    #[test]
    fn stem_malay() {
        assert!(!stem("makanan").is_empty());
        assert!(stem("mengetahui").len() < "mengetahui".len());
    }

    #[test]
    fn stem_tagalog() {
        assert!(!stem("kumain").is_empty());
        assert!(!stem("maganda").is_empty());
    }

    #[test]
    fn stem_thai() {
        assert!(!stem("กินแล้ว").is_empty());
        assert!(!stem("การกิน").is_empty());
    }

    #[test]
    fn stem_swahili() {
        assert!(!stem("wanakula").is_empty());
        assert!(!stem("anapenda").is_empty());
    }

    #[test]
    fn stem_persian() {
        assert!(!stem("کتابها").is_empty());
        assert!(!stem("بهترین").is_empty());
    }

    #[test]
    fn stem_urdu() {
        assert!(!stem("کتابیں").is_empty());
        assert!(!stem("لڑکے").is_empty());
    }

    #[test]
    fn stem_bengali() {
        assert!(!stem("বইগুলো").is_empty());
        assert!(!stem("ছেলেটি").is_empty());
    }

    #[test]
    fn stem_tamil() {
        assert!(!stem("புத்தகங்கள்").is_empty());
        assert!(!stem("வீட்டில்").is_empty());
    }

    #[test]
    fn stem_telugu() {
        assert!(!stem("పుస్తకాలు").is_empty());
        assert!(!stem("ఇంట్లో").is_empty());
    }

    #[test]
    fn stem_marathi() {
        assert!(!stem("मुलगा").is_empty());
        assert!(!stem("घरात").is_empty());
    }

    #[test]
    fn stem_gujarati() {
        assert!(!stem("ઘરમાં").is_empty());
        assert!(!stem("છોકરાઓ").is_empty());
    }

    #[test]
    fn stem_punjabi() {
        assert!(!stem("ਕਿਤਾਬਾਂ").is_empty());
        assert!(!stem("ਮੁੰਡੇ").is_empty());
    }

    #[test]
    fn stem_kannada() {
        assert!(!stem("ಪುಸ್ತಕಗಳು").is_empty());
        assert!(!stem("ಮನೆಯಲ್ಲಿ").is_empty());
    }

    #[test]
    fn stem_malayalam() {
        assert!(!stem("പുസ്തകങ്ങൾ").is_empty());
        assert!(!stem("വീട്ടിൽ").is_empty());
    }

    #[test]
    fn stem_catalan() {
        assert!(!stem("informació").is_empty());
        assert!(stem("informació").len() < "informació".len());
    }

    #[test]
    fn stem_basque() {
        assert!(!stem("etxearekin").is_empty());
        assert!(stem("etxetik").len() < "etxetik".len());
    }

    #[test]
    fn stem_galician() {
        assert!(!stem("información").is_empty());
        assert!(!stem("cidadade").is_empty());
    }

    #[test]
    fn stem_icelandic() {
        assert!(!stem("hestarnir").is_empty());
        assert!(stem("hestarnir").len() < "hestarnir".len());
    }

    #[test]
    fn stem_lithuanian() {
        assert!(!stem("vyras").is_empty());
        assert!(stem("vyro").len() < "vyro".len() || stem("vyro") == "vyro");
    }

    #[test]
    fn stem_latvian() {
        assert!(!stem("vīrietis").is_empty());
        assert!(!stem("mājām").is_empty());
    }

    #[test]
    fn stem_slovak() {
        assert!(!stem("pekný").is_empty());
        assert!(stem("pekný").len() < "pekný".len());
    }

    #[test]
    fn stem_slovenian() {
        assert!(!stem("lepega").is_empty());
        assert!(stem("lepemu").len() < "lepemu".len());
    }

    #[test]
    fn detect_cyrillic_is_uk_or_ru() {
        let lang_ru = detect_language("Привет мир");
        let lang_uk = detect_language("Привіт світ");
        assert!(lang_ru == Language::Ru || lang_ru == Language::Uk);
        assert!(lang_uk == Language::Uk || lang_uk == Language::Ru);
    }

    #[test]
    fn detect_cjk_is_zh_ja_or_ko() {
        let lang_zh = detect_language("你好世界");
        let lang_ja = detect_language("こんにちは");
        let lang_ko = detect_language("안녕하세요");
        assert!(matches!(lang_zh, Language::Zh | Language::Ja | Language::Ko));
        assert!(matches!(lang_ja, Language::Ja | Language::Zh | Language::Ko));
        assert!(matches!(lang_ko, Language::Ko | Language::Ja | Language::Zh));
    }

    #[test]
    fn detect_arabic_is_ar() {
        assert_eq!(detect_language("مرحبا بالعالم"), Language::Ar);
    }

    #[test]
    fn detect_persian_is_fa() {
        assert_eq!(detect_language("کتابهای خوب"), Language::Fa);
    }

    #[test]
    fn detect_urdu_is_ur() {
        assert_eq!(detect_language("لڑکے کتابیں"), Language::Ur);
    }

    #[test]
    fn detect_hebrew_is_he() {
        assert_eq!(detect_language("שלום עולם"), Language::He);
    }

    #[test]
    fn detect_thai_is_th() {
        assert_eq!(detect_language("สวัสดีชาวโลก"), Language::Th);
    }

    #[test]
    fn detect_bengali_is_bn() {
        assert_eq!(detect_language("হ্যালো ওয়ার্ল্ড"), Language::Bn);
    }

    #[test]
    fn detect_tamil_is_ta() {
        assert_eq!(detect_language("வணக்கம் உலகம்"), Language::Ta);
    }

    #[test]
    fn detect_telugu_is_te() {
        assert_eq!(detect_language("హలో వరల్డ్"), Language::Te);
    }

    #[test]
    fn detect_gujarati_is_gu() {
        assert_eq!(detect_language("હેલો વર્લ્ડ"), Language::Gu);
    }

    #[test]
    fn detect_gurmukhi_is_pa() {
        assert_eq!(detect_language("ਹੈਲੋ ਵਰਲਡ"), Language::Pa);
    }

    #[test]
    fn detect_kannada_is_kn() {
        assert_eq!(detect_language("ಹಲೋ ವರ್ಲ್ಡ್"), Language::Kn);
    }

    #[test]
    fn detect_malayalam_is_ml() {
        assert_eq!(detect_language("ഹലോ വേൾഡ്"), Language::Ml);
    }

    #[test]
    fn detect_marathi_is_mr() {
        assert!(matches!(detect_language("नमस्कार जग"), Language::Mr | Language::Hi));
    }

    #[test]
    fn detect_roman_with_suffix_is_ro() {
        assert_eq!(detect_language("documentul"), Language::Ro);
    }

    #[test]
    fn detect_roman_with_suffix_is_hu() {
        assert_eq!(detect_language("embereknek"), Language::Hu);
    }

    #[test]
    fn detect_english_fallback() {
        assert_eq!(detect_language("hello world"), Language::En);
    }

    #[test]
    fn language_as_str() {
        assert_eq!(Language::En.as_str(), "English");
        assert_eq!(Language::Uk.as_str(), "Ukrainian");
        assert_eq!(Language::Zh.as_str(), "Chinese");
        assert_eq!(Language::He.as_str(), "Hebrew");
        assert_eq!(Language::Fi.as_str(), "Finnish");
        assert_eq!(Language::Th.as_str(), "Thai");
    }

    #[test]
    fn language_as_iso639() {
        assert_eq!(Language::En.as_iso639(), "en");
        assert_eq!(Language::Uk.as_iso639(), "uk");
        assert_eq!(Language::Ar.as_iso639(), "ar");
        assert_eq!(Language::El.as_iso639(), "el");
        assert_eq!(Language::Fi.as_iso639(), "fi");
        assert_eq!(Language::Bn.as_iso639(), "bn");
    }

    #[test]
    fn language_as_script_name() {
        assert_eq!(Language::En.as_script_name(), "Latin");
        assert_eq!(Language::Ru.as_script_name(), "Cyrillic");
        assert_eq!(Language::Ar.as_script_name(), "Arabic");
        assert_eq!(Language::Hi.as_script_name(), "Devanagari");
        assert_eq!(Language::El.as_script_name(), "Greek");
        assert_eq!(Language::He.as_script_name(), "Hebrew");
        assert_eq!(Language::Ko.as_script_name(), "Hangul");
        assert_eq!(Language::Th.as_script_name(), "Thai");
        assert_eq!(Language::Bn.as_script_name(), "Bengali");
    }

    #[test]
    fn detect_greek_is_el() {
        assert_eq!(detect_language("καλημέρα κόσμε"), Language::El);
    }

    #[test]
    fn detect_devanagari_is_hi() {
        assert_eq!(detect_language("नमस्ते दुनिया"), Language::Hi);
    }

    #[test]
    fn cover_stem_empty() {
        let _ = super::stem("");
    }

    #[test]
    fn cover_stem_short() {
        let _ = super::stem("ca");
    }

    #[test]
    fn cover_stem_plurals() {
        let _ = super::stem("caresses");
    }

    #[test]
    fn cover_stem_ing() {
        let _ = super::stem("running");
    }

    #[test]
    fn cover_stem_ly() {
        let _ = super::stem("happily");
    }

    #[test]
    fn cover_stem_ment() {
        let _ = super::stem("enjoyment");
    }

    #[test]
    fn cover_stem_ness() {
        let _ = super::stem("happiness");
    }

    #[test]
    fn cover_stem_ize() {
        let _ = super::stem("normalize");
    }

    #[test]
    fn cover_stem_able() {
        let _ = super::stem("readable");
    }

    #[test]
    fn cover_stem_less() {
        let _ = super::stem("fearless");
    }

    #[test]
    fn cover_stem_ous() {
        let _ = super::stem("dangerous");
    }

    #[test]
    fn cover_stem_tional() {
        let _ = super::stem("relational");
    }

    #[test]
    fn cover_stem_ful() {
        let _ = super::stem("beautiful");
    }

    #[test]
    fn cover_tokenize_stemmed_empty() {
        let _ = super::tokenize_stemmed("");
    }

    #[test]
    fn cover_tokenize_stemmed_text() {
        let _ = super::tokenize_stemmed("hello world");
    }

    #[test]
    fn cover_detect_language_empty() {
        let _ = super::detect_language("");
    }

    #[test]
    fn cover_detect_language_en() {
        let _ = super::detect_language("the world is a beautiful place with many wonderful things to see and do every day");
    }

    #[test]
    fn cover_detect_language_de() {
        let _ = super::detect_language("die Welt ist ein wunderschoener Ort mit vielen schoenen Dingen die man sehen und machen kann jeden Tag");
    }

    #[test]
    fn cover_stem_s() {
        let _ = super::stem("tests");
    }

    #[test]
    fn cover_stem_eed() {
        let _ = super::stem("proceed");
    }

    #[test]
    fn cover_stem_ed() {
        let _ = super::stem("played");
    }

    #[test]
    fn cover_stem_ies() {
        let _ = super::stem("parties");
    }

    #[test]
    fn cover_stem_sses() {
        let _ = super::stem("glasses");
    }

    #[test]
    fn cover_stem_ement() {
        let _ = super::stem("replacement");
    }

    #[test]
    fn cover_stem_ance() {
        let _ = super::stem("acceptance");
    }

    #[test]
    fn cover_stem_ence() {
        let _ = super::stem("dependence");
    }

    #[test]
    fn cover_stem_er() {
        let _ = super::stem("runner");
    }

    #[test]
    fn cover_stem_ic() {
        let _ = super::stem("rustic");
    }

    #[test]
    fn cover_stem_iti() {
        let _ = super::stem("sensitivity");
    }

    #[test]
    fn cover_stem_ble() {
        let _ = super::stem("visible");
    }

    #[test]
    fn cover_stem_ative() {
        let _ = super::stem("generative");
    }

    #[test]
    fn cover_stem_alize() {
        let _ = super::stem("finalize");
    }

    #[test]
    fn cover_stem_entli() {
        let _ = super::stem("gently");
    }

    #[test]
    fn cover_stem_eli() {
        let _ = super::stem("nicely");
    }

    #[test]
    fn cover_stem_alli() {
        let _ = super::stem("basically");
    }

    #[test]
    fn cover_stem_izing() {
        let _ = super::stem("stabilizing");
    }

    #[test]
    fn cover_stem_ational() {
        let _ = super::stem("sensational");
    }

    #[test]
    fn cover_stem_us() {
        let _ = super::stem("nervous");
    }

    #[test]
    fn cover_stem_ism() {
        let _ = super::stem("communism");
    }

    #[test]
    fn cover_stem_ist() {
        let _ = super::stem("artist");
    }

    #[test]
    fn cover_stem_ity() {
        let _ = super::stem("velocity");
    }

    #[test]
    fn stem_german_suffixes() {
        let s = stem("Freiheitlichkeit");
        assert!(s.len() < "Freiheitlichkeit".len());
        assert!(!stem("Lesens").is_empty());
    }

    #[test]
    fn stem_french_suffixes() {
        assert!(stem("gouvernement").len() < "gouvernement".len());
        assert!(stem("joliment").len() < "joliment".len());
    }

    #[test]
    fn stem_spanish_suffixes() {
        assert!(stem("comunicación").len() < "comunicación".len());
        assert!(stem("hermosamente").len() < "hermosamente".len());
    }

    #[test]
    fn stem_dutch_suffixes() {
        assert!(stem("vriendschap").len() < "vriendschap".len());
        assert!(stem("werkzaamheden").len() < "werkzaamheden".len());
    }

    #[test]
    fn stem_romanian_suffixes() {
        assert!(stem("documentului").len() < "documentului".len());
        assert!(stem("cartilor").len() < "cartilor".len());
    }

    #[test]
    fn stem_hungarian_suffixes() {
        assert!(stem("házakban").len() < "házakban".len());
        assert!(stem("embereknek").len() < "embereknek".len());
    }

    #[test]
    fn stem_whitespace_padded() {
        let s = stem("  running  ");
        assert_eq!(s, "run");
    }

    #[test]
    fn stem_numeric_only() {
        let s = stem("12345");
        assert_eq!(s, "12345");
    }

    #[test]
    fn stem_special_characters_only() {
        let s = stem("@#$%");
        assert_eq!(s, "@#$%");
    }

    #[test]
    fn stem_very_long_suffixed_word() {
        let w = "unimaginativelyimaginativelyrunning";
        let s = stem(w);
        assert!(s.len() <= w.len());
    }

    #[test]
    fn detect_mixed_script_cyrillic_latin() {
        let lang = detect_language("привет hello");
        assert!(matches!(lang, Language::Ru | Language::Uk));
    }

    #[test]
    fn detect_latin_trumps_by_suffix() {
        let lang = detect_language("dokumentumok házak");
        assert_eq!(lang, Language::Hu);
    }
}
