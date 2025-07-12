use crate::nfa::{Transition, TransitionModifier};

use super::nfa::Nfa;

#[derive(Debug, Clone)]
pub enum ParseElement {
    Literal(char), // a single character
    Wildcard,      // . matches any character

    Star,            // * matches 0 or more times
    Plus,            // + matches 1 or more times
    Question,        // ? matches 0 or 1 times
    Range(u64, u64), // a{3,5} matches aaa, aaaa, aaaaa

    Union, // |

    Group(Vec<ParseElement>),   // (...)
    Bracket(Vec<char>),         // [A-Za-z]
    NegativeBracket(Vec<char>), // [^A-Za-z]

    Class(char),        // \w \W \d \D \s \S \b
    BackReference(u64), //\n where n>=1, POSIX regex only mandates 1-9
}

pub fn lex(input: String) -> Vec<ParseElement> {
    let mut iter = input.chars().peekable();
    let mut stack = Vec::new();
    let mut curr = Vec::new();

    while iter.peek().is_some() {
        match iter.next().unwrap() {
            '.' => curr.push(ParseElement::Wildcard),
            '*' => curr.push(ParseElement::Star),
            '+' => curr.push(ParseElement::Plus),
            '?' => curr.push(ParseElement::Question),
            '{' => {
                // range
                let (mut min, mut max) = (0u64, 0u64);
                while iter.peek().unwrap().is_digit(10) {
                    min *= 10;
                    min += iter.next().unwrap().to_digit(10).unwrap() as u64;
                }

                if !matches!(iter.next().unwrap(), ',') {
                    panic!("Expected comma in range!")
                }

                // TODO allow whitespace?
                while iter.peek().unwrap().is_digit(10) {
                    max *= 10;
                    max += iter.next().unwrap().to_digit(10).unwrap() as u64;
                }

                if !matches!(iter.next().unwrap(), '}') {
                    panic!("Expected close curly in range!");
                }
                curr.push(ParseElement::Range(min, max));
            }

            '|' => curr.push(ParseElement::Union),

            '(' => {
                // new group
                stack.push(curr.clone());
                curr.clear();
            }
            ')' => {
                // close group
                let group = ParseElement::Group(curr.clone());
                curr.clear();
                curr = stack.pop().unwrap();
                curr.push(group);
            }
            '[' => {
                // bracket
                let mut negative = false;
                if matches!(iter.peek().unwrap(), '^') {
                    let _ = iter.next();
                    negative = true;
                }

                let mut values = Vec::new();

                while !matches!(iter.peek().unwrap(), ']') {
                    let curr = iter.next().unwrap();
                    match curr {
                        '\\' => match iter.peek().unwrap() {
                            '.' | '*' | '+' | '?' | '{' | '}' | '|' | '^' | '$' | '(' | ')'
                            | '[' | ']' | '-' | '\\' => {
                                values.push(iter.next().unwrap());
                            }
                            _ => {
                                panic!("Unknown escape in brackets!");
                            }
                        },
                        '-' => {
                            let prev = values.pop().unwrap();
                            let end = iter.next().unwrap();
                            for c in prev..=end {
                                values.push(c);
                            }
                        }
                        c => {
                            values.push(c);
                        }
                    }
                }

                // consume closing bracket
                let _ = iter.next();

                if negative {
                    curr.push(ParseElement::NegativeBracket(values));
                } else {
                    curr.push(ParseElement::Bracket(values));
                }
            }

            '\\' => {
                // Escaped character
                let next = iter.next().unwrap();

                match next {
                    'w' | 'W' | 'd' | 'D' | 's' | 'S' | 'b' => curr.push(ParseElement::Class(next)),
                    '0'..='9' => {
                        // digits
                        let mut n: u64 = next.to_digit(10).unwrap() as u64;
                        while iter.peek().unwrap().is_digit(10) {
                            n *= 10;
                            n += iter.next().unwrap().to_digit(10).unwrap() as u64;
                        }
                        curr.push(ParseElement::BackReference(n));
                    }
                    '.' | '*' | '+' | '?' | '{' | '}' | '|' | '^' | '$' | '(' | ')' | '[' | ']'
                    | '-' | '\\' => {
                        curr.push(ParseElement::Literal(next));
                    }
                    _ => {
                        panic!("Unknown escape sequence {}", next);
                    }
                }
            }
            c => curr.push(ParseElement::Literal(c)),
        }
    }

    if stack.len() != 0 {
        panic!("Unfinished stack!");
    }

    curr
}

pub fn parse(toks: Vec<ParseElement>) -> Nfa {
    let mut curr_nfa = Nfa::empty();
    let mut union_stack = Vec::new();

    let mut tok_iter = toks.iter().peekable();

    while let Some(tok) = tok_iter.next() {
        let modifier = match tok_iter.peek() {
            Some(ParseElement::Star) => {
                let _ = tok_iter.next();
                Some(TransitionModifier::Star)
            }
            Some(ParseElement::Plus) => {
                let _ = tok_iter.next();
                Some(TransitionModifier::Plus)
            }
            Some(ParseElement::Question) => {
                let _ = tok_iter.next();
                Some(TransitionModifier::Question)
            }
            Some(ParseElement::Range(mi, ma)) => {
                let _ = tok_iter.next();
                Some(TransitionModifier::Range(*mi, *ma))
            }
            _ => None,
        };
        match tok {
            ParseElement::Literal(c) => {
                curr_nfa.concat(&mut Nfa::new(Transition::Literal(*c), modifier));
            }
            ParseElement::Union => {
                union_stack.push(curr_nfa);
                curr_nfa = Nfa::empty();
            }
            ParseElement::Wildcard => {
                curr_nfa.concat(&mut Nfa::new(Transition::Wildcard, modifier));
            }
            ParseElement::Bracket(chars) => {
                let mut chars = chars.clone();
                let mut new_nfa = Nfa::new(Transition::Literal(chars.pop().unwrap()), None);
                while chars.len() > 0 {
                    new_nfa.union(&mut Nfa::new(
                        Transition::Literal(chars.pop().unwrap()),
                        None,
                    ));
                }

                new_nfa.add_modifier(modifier);

                curr_nfa.concat(&mut new_nfa);
            }
            ParseElement::Star
            | ParseElement::Plus
            | ParseElement::Question
            | ParseElement::Range(_, _) => {
                panic!("Unexpected modifier!");
            }
            _ => panic!("Unknown token {:?}", tok),
        }
    }

    while union_stack.len() > 0 {
        curr_nfa.union(&mut union_stack.pop().unwrap());
    }
    curr_nfa
}
