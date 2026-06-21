#[derive(Debug, Clone)]
pub enum RepeatType {
    Exactly(u32),
    AtLeast(u32),
    Between(u32, u32),
}

#[derive(Debug, Clone)]
pub struct CharClassItem {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct CharClass {
    pub negated: bool,
    pub items: Vec<CharClassItem>,
}

#[derive(Debug, Clone)]
pub enum RegexType {
    Char(u32),
    CharClass(CharClass),
    Concat(Box<RegexType>, Box<RegexType>),
    Alt(Box<RegexType>, Box<RegexType>),
    Star(Box<RegexType>),
    Plus(Box<RegexType>),
    Opt(Box<RegexType>),
    Repeat(Box<RegexType>, RepeatType),
}
