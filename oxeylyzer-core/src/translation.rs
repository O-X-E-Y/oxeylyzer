use smartstring::{SmartString, Compact, LazyCompact};
use anyhow::Result;
use fxhash::FxHashMap;

#[derive(Clone)]
pub struct Translator {
    pub table: FxHashMap<char, SmartString<Compact>>,
    pub is_raw: bool,
    pub(crate) is_empty: bool
}

impl Default for Translator {
    fn default() -> Self {
        let mut translator = Translator::new();
        translator
		    .default_formatting()
		    .build()
    }
}

impl std::ops::Add for Translator {
    type Output = Self;

    ///the table of the FIRST translator takes priority over the SECOND.
    fn add(mut self, rhs: Self) -> Self::Output {
        self.is_raw |= rhs.is_raw;
        self.is_empty &= rhs.is_empty;

        if !self.is_empty {
            let base = &SmartString::<Compact>::from(" ");
            for (from, to) in rhs.table {
                let original = self.table.get(&from);
                if original.is_none() || original == Some(base) {
                    self.table.insert(from, to);
                }
            }
            self
        } else {
            self.table = rhs.table;
            self
        }
    }
}

impl Translator {
    pub fn new() -> TranslatorBuilder {
        TranslatorBuilder {
            table: FxHashMap::default(),
            is_raw: false
        }
    }

    #[allow(dead_code)]
    pub(crate) fn language(language: &str) -> Result<Self> {
        Ok(Self::new()
            .language(language)?
            .build())
    }

     #[allow(dead_code)]
    pub(crate) fn language_or_default(language: &str) -> Self {
        if let Ok(t) = Self::language(language) {
            t
        } else {
            Self::default()
        }
    }

     #[allow(dead_code)]
    pub(crate) fn language_or_raw(language: &str) -> Self {
        if let Ok(t) = Self::language(language) {
            t
        } else {
            Self::raw(true)
        }
    }

    pub fn raw(unshift_chars: bool) -> Self {
        Translator::new()
            .raw(unshift_chars)
            .build()
    }

    pub fn translate(&self, s: &str) -> SmartString<LazyCompact> {
        let mut res = SmartString::<LazyCompact>::new();

        for c in s.chars() {
            if let Some(replacement) = self.table.get(&c) {
                res.push_str(replacement);
            } else {
                res.push(' ');
            }
        }
        res
	}

    pub fn translate_arr(&self, arr: &[char]) -> SmartString<LazyCompact> {
        let mut res = SmartString::<LazyCompact>::new();

        for c in arr.into_iter() {
            if let Some(replacement) = self.table.get(c) {
                res.push_str(replacement);
            } else {
                res.push(' ');
            }
        }
        res
    }
}

pub struct TranslatorBuilder {
    table: FxHashMap<char, SmartString<Compact>>,
    is_raw: bool
}

impl TranslatorBuilder {
    pub fn to_nothing(&mut self, to_nothing: &str) -> &mut Self {
        for c in to_nothing.chars() {
            self.table.insert(c, SmartString::<Compact>::from(""));
        }
        self
    }

    pub fn to_space(&mut self, to_string: &str) -> &mut Self {
        for c in to_string.chars() {
            self.table.insert(c, SmartString::<Compact>::from(" "));
        }
        self
    }

    pub fn many_different_to_one(&mut self, from: &str, to: char) -> &mut Self {
        for c in from.chars() {
            self.table.insert(c, SmartString::<Compact>::from(to));
        }
        self
    }

    pub fn keep_one(&mut self, keep: char) -> &mut Self {
        self.table.insert(keep, SmartString::<Compact>::from(keep));
        self
    }

    pub fn keep(&mut self, keep: &str) -> &mut Self {
        for c in keep.chars() {
            self.table.insert(c, SmartString::<Compact>::from(c));
        }
        self
    }

    pub fn one_to_one(&mut self, from: &str, to: &str) -> &mut Self {
        assert_eq!(from.chars().count(), to.chars().count());

        for (f, t) in from.chars().zip(to.chars()) {
            self.table.insert(f, SmartString::<Compact>::from(t));
        }
        self
    }

    pub fn one_multiple(&mut self, from: char, to: &str) -> &mut Self {
        self.table.insert(from, SmartString::<Compact>::from(to));
        self
    }

    #[inline(always)]
    fn one_multiple_smartstr(&mut self, from: char, to: SmartString<Compact>) -> &mut Self {
        self.table.insert(from, to);
        self
    }

    pub fn to_multiple(&mut self, trans: Vec<(char, &str)>) -> &mut Self {
        for (f, t) in trans {
            self.table.insert(f, SmartString::<Compact>::from(t));
        }
        self
    }

    pub fn to_multiple_string(&mut self, trans: &Vec<(char, String)>) -> &mut Self {
        for (f, t) in trans {
            self.table.insert(*f, SmartString::<Compact>::from(t));
        }
        self
    }

    pub fn letter_to_lowercase(&mut self, letter: char) -> &mut Self {
        self.table.insert(letter, SmartString::<Compact>::from(letter));

        let mut upper_string = letter.to_uppercase();

        if upper_string.clone().count() == 1 {
            let uppercase_letter = upper_string.next().unwrap();
            
            let shifted = SmartString::<Compact>::from_iter([' ', letter]);
            self.one_multiple_smartstr(uppercase_letter, shifted);
        }
        self
    }

    pub fn letters_to_lowercase(&mut self, letters: &str) -> &mut Self {
        for letter in letters.chars() {
            self.letter_to_lowercase(letter);
        }
        self
    }

    pub fn raw(&mut self, unshift_chars: bool) -> &mut Self {
        self.is_raw = true;
        self.normalize_punct();

        if unshift_chars {
            for i in 128u32..75_000 {
                if let Some(c) = char::from_u32(i) {
                    if c.is_alphabetic() {
                        if c.is_lowercase() {
                            self.letter_to_lowercase(c);
                        } else {
                            self.keep_one(c);
                        }
                    } else if !c.is_control() {
                        self.keep_one(c);
                    }
                }
            }
            self.ascii_lower()
        } else {
            for i in 0u32..75_000 {
                if let Some(c) = char::from_u32(i) && !c.is_control() {
                    self.keep_one(c);
                }
            }
            self
        }
    }

    pub fn custom_unshift(&mut self, upper_version: &str, lower_version: &str) -> &mut Self {
        for (upper, lower) in upper_version.chars().zip(lower_version.chars()) {
            let shifted = SmartString::<Compact>::from_iter([' ', lower]);
            self.one_multiple_smartstr(upper, shifted);
        }

        self
            .keep(lower_version)
    }

    pub(crate) fn punct_lower(&mut self) -> &mut Self {
        for (upper, lower) in "{}?+_|\"<>:~".chars().zip("[]/=-\\',.;`".chars()) {
            let shifted = String::from_iter([' ', lower]);
            self.one_multiple(upper, shifted.as_str());
        }

        self
            .keep("[]/=-\\',.;`")
    }

    pub(crate) fn alphabet_lower(&mut self) -> &mut Self {
        self.letters_to_lowercase("abcdefghijklmnopqrstuvwxyz")
    }

    pub(crate) fn ascii_lower(&mut self) -> &mut Self {
        self
            .punct_lower()
            .alphabet_lower()
    }

    pub(crate) fn normalize_punct(&mut self) -> &mut Self {
        self
            .one_to_one("???????????????????????????","'''/''''-''")
            .one_multiple('???', "...")
    }

    pub(crate) fn default_formatting(&mut self) -> &mut Self {
        self
            .ascii_lower()
            .normalize_punct()
    }

    pub(crate) fn language(&mut self, language: &str) -> Result<&mut Self> {
        self.default_formatting();
        match language.to_lowercase().as_str() {
            "akl" | "english" | "english2" | "toki_pona" | "indonesian" | "reddit" => Ok(self),
            "albanian" => Ok(self
                .letters_to_lowercase("????")
            ),
            "bokmal" | "nynorsk" | "danish" => Ok(self
                .letters_to_lowercase("??????")
            ),
            "czech" => Ok(self
                .to_multiple(vec![
                    ('??', "*c"), ('??', "*d"), ('??', "*x"), ('??', "*n"), ('??', "*o"), ('??', "*r"),
                    ('??', "*s"), ('??', "*t"), ('??', "*u"), ('??', "*b"), ('??', "*y"), ('??', "*z"),
                    ('??', "*c"), ('??', "*d"), ('??', "*x"), ('??', "*n"), ('??', "*o"), ('??', "*r"),
                    ('??', "*s"), ('??', "*t"), ('??', "*u"), ('??', "*b"), ('??', "*y"), ('??', "*z")
                ])
                .letters_to_lowercase("??????")
            ),
            "dan-en70-30" => Ok(self
                .letters_to_lowercase("??????")
            ),
            "dan-en70-30a" => Ok(self
                .to_multiple(vec![
                    ('??', "*a"), ('??', "*o"), ('??', "*e")
                ])
            ),
            "dutch" => Ok(self.letters_to_lowercase("????????????????")),
            "dutch_repeat" => Ok(self.letters_to_lowercase("????????????????@")),
            "english_repeat" => Ok(self.keep("@")),
            "english_th" => Ok(self.letters_to_lowercase("??")),
            "esperanto" => Ok(self
                .letters_to_lowercase("????????????")
            ),
            "finnish" => Ok(self
                .letters_to_lowercase("??????")
            ),
            "finnish_repeat" => Ok(self
                .letters_to_lowercase("??????@")
            ),
            "french" | "french_qu" | "test" => Ok(self
                .to_multiple(vec![
                    ('??', "*c"), ('??', "*c"), ('??', "oe"),    ('??', "* a"), ('??', "* a"), ('??', "* e"),
                    ('??', "* e"), ('??', "* i"), ('??', "* i"), ('??', "* i"), ('??', "* o"), ('??', "* o"),
                    ('??', "* o"), ('??', "* u"), ('??', "* u"), ('??', "* u"), ('??', "* a"), ('??', "* a"),
                    ('??', "* e"), ('??', "* e"), ('??', "* i"), ('??', "* i"), ('??', "* i"), ('??', "* o"),
                    ('??', "* o"), ('??', "* o"), ('??', "* u"), ('??', "* u"), ('??', "* u"), ('??', "* a"),
                    ('??', "* e"), ('??', "* i"), ('??', "* o"), ('??', "* u"), ('??', "* a"), ('??', "* e"),
                    ('??', "* i"), ('??', "* o"), ('??', "* u")
                ])
                .letters_to_lowercase("????")
            ),
            "german" => Ok(self.letters_to_lowercase("????????")),
            "hungarian" => Ok(self
                .to_multiple(vec![
                    ('??', "*i"), ('??', "*u"), ('??', "* u"), ('??', "* u"), ('??', "*i"), ('??', "*u"),
                    ('??', "* u"), ('??', "* u")
                ])
                .letters_to_lowercase("??????????")
            ),
            "italian" => Ok(self
                .to_multiple(vec![
                    ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"), ('??', "*a"),
                    ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u")
                ])
            ),
            "korean" => Ok(self
                .to_space("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .keep("??????????????????????????????????????????????????????????????????????????????")
                .one_to_one("?????????????????????", "?????????????????????")
                .to_multiple(vec![
                    ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "?????????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "?????????"), ('???', "??????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "?????????"), ('???', "?????????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "???"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"), ('???', "??????"),
                    ('???', "??????"), ('???', "??????"), ('???', "???"), ('???', "???")
                ])
            ),
            "luxembourgish" => Ok(self
                .to_multiple(vec![
                    ('??', " "), ('e', " ??"), ('u', " ??"), ('i', " ??"), ('s', " ??"), ('d', " ???"),
                    ('c', " ??")
                ])
            ),
            "polish" => Ok(self
                .to_multiple(vec![
                    ('??', "*a"), ('??', "*o"), ('??', "*z"), ('??', "*s"), ('??', "*c"), ('??', "*n")
                ])
                .letters_to_lowercase("??????")
            ),
            "russian" => Ok(self
                .letters_to_lowercase("??????????????????????????????????????????????????????????????????")
                .to_space("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
            ),
            "spanish" => Ok(self
                .to_multiple(vec![
                    ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"), ('??', "*y"),
                    ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"), ('??', "*y"),
                    ('??', "*n"), ('??', "*n")
                ])
            ),
            "swedish" => Ok(
                self.letters_to_lowercase("??????")
            ),
            "welsh" => Ok(self
                .to_multiple(vec![
                    ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"), ('??', "*w"),
                    ('??', "*y"), ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"),
                    ('??', "*w"), ('??', "*y")
                ])
                .letters_to_lowercase("?????")
            ),
            "welsh_pure" => Ok(self
                .to_multiple(vec![
                    ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"), ('??', "*w"),
                    ('??', "*y"), ('??', "*a"), ('??', "*e"), ('??', "*i"), ('??', "*o"), ('??', "*u"),
                    ('??', "*w"), ('??', "*y")
                ])
            ),
            _ => Err(anyhow::format_err!("This language is not available. You'll have to make your own formatter, sorry!"))
        }
    }

    pub fn build(&mut self) -> Translator {
        Translator {
            is_empty: self.table.len() == 0,
            table: std::mem::take(&mut self.table),
            is_raw: self.is_raw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHABET: &str =       "abcdefghijklmnopqrstuvwxyz";
    const ALPHABET_UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const ALPHABET_SHIFTED: &str = " a b c d e f g h i j k l m n o p q r s t u v w x y z";
    const NUMS: &str =           "1234567890";
    const NUMS_UPPER: &str =     "!@#$%^&*()";
    const SYMBOLS: &str =        " ` [ ] / = - \\ ' , . ;";
    const SYMBOLS_SHIFTED: &str =  "~{}?+_|\"<>:";
    
    #[test]
    fn test_translate_default() {
        let translator = Translator::default();

        assert_eq!(translator.translate(ALPHABET), ALPHABET);
        assert_eq!(translator.translate(ALPHABET_SHIFTED), translator.translate(ALPHABET_UPPER));
        assert_eq!(translator.translate(NUMS), "          ");
        assert_eq!(translator.translate(NUMS_UPPER), "          ");
        assert_eq!(translator.translate(SYMBOLS), translator.translate(SYMBOLS_SHIFTED));
        assert_eq!(translator.translate("????"), "  ");
        assert_eq!(translator.translate("???"), "...");
        assert_eq!(translator.translate("???????????????????????????"), "'''/''''-''");
    }

    #[test]
    fn test_keep_all() {
        let translator = Translator::new()
            .raw(false)
            .build();
        
        assert_eq!(translator.translate("??Aamong us"), "??Aamong us");
        assert_eq!(translator.translate(NUMS), NUMS);
    }

    #[test]
    fn test_multiple() {
        let translator = Translator::new()
            .to_multiple(vec![('??', "* z")])
            .letters_to_lowercase("a??")
            .build();
        
        assert_eq!(translator.translate("??Aa?? ??"), "* z aa  ??");
    }

    #[test]
    fn add_translators_together() {
        let t1 = Translator::new()
            .one_multiple('a', "abc")
            .one_to_one("b", "_")
            .build();
        let t2 = Translator::new()
            .one_multiple('c', "cba")
            .one_to_one("b", "-")
            .build();

        let t3 = t1.clone() + t2.clone();
        let t4 = t2 + t1.clone();
        
        assert_eq!(t3.translate("abcd"), "abc_cba ");
        assert_eq!(t4.translate("abcd"), "abc-cba ");

        let t_empty = Translator::new().build();
        let t5 = t1.clone() + t_empty.clone();
        let t6 = t_empty + t1;
        
        assert_eq!(t5.translate("abcd"), "abc_  ");
        assert_eq!(t5.translate("abcd"), t6.translate("abcd"));
    }
}