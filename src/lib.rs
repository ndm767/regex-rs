pub mod dfa;
pub mod nfa;
pub mod parse;
pub mod transition_table;

use crate::dfa::{Dfa, SimError};
use crate::parse::{lex, parse};

pub fn compile_regex(input: &str) -> Dfa {
    let nfa = parse(lex(input.to_string()));

    let mut dfa = Dfa::from_nfa(nfa);
    dfa.minimize();

    dfa
}

pub fn test_string(input: &str, dfa: &Dfa) -> Result<(), SimError> {
    dfa.simulate(input.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brackets_char_classes() {
        let r = compile_regex("\\w");

        for c in 'a'..='z' {
            assert_eq!(test_string(String::from(c).as_str(), &r), Ok(()));
        }

        for c in 'A'..='Z' {
            assert_eq!(test_string(String::from(c).as_str(), &r), Ok(()));
        }

        for c in '0'..='9' {
            assert_eq!(test_string(String::from(c).as_str(), &r), Ok(()));
        }

        assert_eq!(test_string(String::from('_').as_str(), &r), Ok(()));

        assert_eq!(test_string(" ", &r), Err(SimError::NoMatch(' ')));
    }

    #[test]
    fn test_wildcard_simple() {
        let r = compile_regex("a.b");

        assert_eq!(test_string("abb", &r), Ok(()));
        assert_eq!(test_string("axb", &r), Ok(()));
        assert_eq!(test_string("ab", &r), Err(SimError::EndOfString));
        assert_eq!(test_string("axby", &r), Err(SimError::Premature));
    }

    #[test]
    fn test_ranges() {
        // normal range
        let r1 = compile_regex("a{3, 5}");

        assert_eq!(test_string("", &r1), Err(SimError::EndOfString));
        assert_eq!(test_string("a", &r1), Err(SimError::EndOfString));
        assert_eq!(test_string("aa", &r1), Err(SimError::EndOfString));
        assert_eq!(test_string("aaa", &r1), Ok(()));
        assert_eq!(test_string("aaaa", &r1), Ok(()));
        assert_eq!(test_string("aaaaa", &r1), Ok(()));
        assert_eq!(test_string("aaaaaa", &r1), Err(SimError::Premature));

        // exact repetition
        let r2 = compile_regex("a{3}");

        assert_eq!(test_string("aa", &r2), Err(SimError::EndOfString));
        assert_eq!(test_string("aaa", &r2), Ok(()));
        assert_eq!(test_string("aaaa", &r2), Err(SimError::Premature));

        // open range
        let r3 = compile_regex("a{3,}");
        assert_eq!(test_string("aa", &r3), Err(SimError::EndOfString));
        assert_eq!(test_string("aaa", &r3), Ok(()));
        assert_eq!(test_string("aaaa", &r3), Ok(()));
        assert_eq!(test_string("aaaaa", &r3), Ok(()));
    }

    #[test]
    fn test_repetition() {
        // *
        let r1 = compile_regex("a*");

        assert_eq!(test_string("", &r1), Ok(()));
        assert_eq!(test_string("a", &r1), Ok(()));
        assert_eq!(test_string("aa", &r1), Ok(()));
        assert_eq!(test_string("aaa", &r1), Ok(()));
        assert_eq!(test_string("aaab", &r1), Err(SimError::Premature));

        // +
        let r2 = compile_regex("a+");

        assert_eq!(test_string("", &r2), Err(SimError::EndOfString));
        assert_eq!(test_string("a", &r2), Ok(()));
        assert_eq!(test_string("aa", &r2), Ok(()));

        // ?
        let r3 = compile_regex("a?");

        assert_eq!(test_string("", &r3), Ok(()));
        assert_eq!(test_string("a", &r3), Ok(()));
        assert_eq!(test_string("aa", &r3), Err(SimError::Premature));
    }

    #[test]
    fn test_union() {
        let r1 = compile_regex("a*|b");

        assert_eq!(test_string("", &r1), Ok(()));
        assert_eq!(test_string("a", &r1), Ok(()));
        assert_eq!(test_string("aa", &r1), Ok(()));
        assert_eq!(test_string("b", &r1), Ok(()));
        assert_eq!(test_string("bb", &r1), Err(SimError::Premature));
        assert_eq!(test_string("ab", &r1), Err(SimError::Premature));

        let r2 = compile_regex("ab|12");
        assert_eq!(test_string("ab", &r2), Ok(()));
        assert_eq!(test_string("12", &r2), Ok(()));
        assert_eq!(test_string("a2", &r2), Err(SimError::NoMatch('2')));
    }

    #[test]
    fn test_group() {
        let r1 = compile_regex("(abc)+");

        assert_eq!(test_string("", &r1), Err(SimError::EndOfString));
        assert_eq!(test_string("abc", &r1), Ok(()));
        assert_eq!(test_string("abcabc", &r1), Ok(()));
        assert_eq!(test_string("abcabcab", &r1), Err(SimError::EndOfString));

        let r2 = compile_regex("((ab)+|(12)*)+");

        assert_eq!(test_string("", &r2), Ok(()));
        assert_eq!(test_string("ab", &r2), Ok(()));
        assert_eq!(test_string("abab12ab12", &r2), Ok(()));
    }

    #[test]
    fn test_backreference() {
        let r1 = compile_regex("(ab+)12\\1*");

        assert_eq!(test_string("ab12ab", &r1), Ok(()));
        assert_eq!(test_string("abbbbbbb12", &r1), Ok(()));
        assert_eq!(test_string("abb12abbbbababb", &r1), Ok(()));

        let r2 = compile_regex("(ab*)+(12?)*\\1?\\2+");
        assert_eq!(test_string("aabbbbbaba121112", &r2), Ok(()));

        let r3 = compile_regex("(1)(2)(3)(4)(5)(6)(7)(8)(9)(10)(11)\\11");
        assert_eq!(test_string("123456789101111", &r3), Ok(()));
    }

    #[test]
    fn test_hex_escape() {
        let r1 = compile_regex("\\x4E");

        assert_eq!(test_string("N", &r1), Ok(()));
        assert_eq!(test_string("n", &r1), Err(SimError::NoMatch('n')));

        let r2 = compile_regex("\\u006e");
        assert_eq!(test_string("n", &r2), Ok(()));
        assert_eq!(test_string("N", &r2), Err(SimError::NoMatch('N')));
    }
}
