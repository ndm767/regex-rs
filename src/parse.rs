use crate::nfa::TransitionModifier;
use crate::transition_table::Transition;

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

    Group(Vec<ParseElement>), // (...)
    Bracket(Vec<char>),       // [A-Za-z]

    BackReference(u64), //\n where n>=1, POSIX regex only mandates 1-9
}

impl ParseElement {
    fn is_modifier(&self) -> bool {
        matches!(
            self,
            Self::Star | Self::Plus | Self::Question | Self::Range(_, _)
        )
    }
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
                // consume until digit
                while !iter.peek().unwrap().is_ascii_digit() {
                    let _ = iter.next();
                }

                // range
                let (mut min, mut max) = (0u64, 0u64);
                while iter.peek().unwrap().is_ascii_digit() {
                    min *= 10;
                    min += iter.next().unwrap().to_digit(10).unwrap() as u64;
                }

                // consume until comma
                while !matches!(iter.next().unwrap(), ',') {}

                // consume until next digit
                while !iter.peek().unwrap().is_ascii_digit() {
                    let _ = iter.next();
                }

                while iter.peek().unwrap().is_ascii_digit() {
                    max *= 10;
                    max += iter.next().unwrap().to_digit(10).unwrap() as u64;
                }

                // consume until close curly
                while !matches!(iter.next().unwrap(), '}') {}

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

                curr.push(ParseElement::Bracket(values));
            }

            '\\' => {
                // Escaped character
                let next = iter.next().unwrap();

                match next {
                    'w' | 'd' | 's' => {
                        // character classes are treated like brackets
                        match next {
                            'w' => {
                                /* [A-Za-z0-9_]  */
                                let mut values: Vec<char> = ('A'..='Z').collect();
                                values.append(&mut ('a'..='z').collect());
                                values.append(&mut ('0'..='9').collect());
                                values.push('_');
                                curr.push(ParseElement::Bracket(values));
                            }
                            'd' => {
                                /* [0-9] */
                                let values: Vec<char> = ('0'..='9').collect();
                                curr.push(ParseElement::Bracket(values));
                            }
                            's' => {
                                /* [ \t] */
                                let values = vec![' ', '\t'];
                                curr.push(ParseElement::Bracket(values));
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                    }
                    '0'..='9' => {
                        // digits
                        let mut n: u64 = next.to_digit(10).unwrap() as u64;
                        while iter.peek().unwrap().is_ascii_digit() {
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
                        panic!("Unknown escape sequence {next}");
                    }
                }
            }
            c => curr.push(ParseElement::Literal(c)),
        }
    }

    if !stack.is_empty() {
        panic!("Unfinished stack!");
    }

    curr
}

pub fn parse(toks: Vec<ParseElement>) -> Nfa {
    let mut curr_nfa = Nfa::empty();

    let mut union_stack = Vec::new();
    let mut groups = Vec::new();

    let mut tok_iter = toks.iter().peekable();

    while let Some(tok) = tok_iter.next() {
        let modifier = match tok_iter.peek() {
            Some(m) if m.is_modifier() => {
                #[allow(suspicious_double_ref_op)]
                let m = m.clone();
                let _ = tok_iter.next();

                Some(match m {
                    ParseElement::Star => TransitionModifier::Star,
                    ParseElement::Plus => TransitionModifier::Plus,
                    ParseElement::Question => TransitionModifier::Question,
                    ParseElement::Range(lo, hi) => TransitionModifier::Range(*lo, *hi),
                    _ => unreachable!(),
                })
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
                while !chars.is_empty() {
                    new_nfa.union(&mut Nfa::new(
                        Transition::Literal(chars.pop().unwrap()),
                        None,
                    ));
                }

                new_nfa.add_modifier(modifier);

                curr_nfa.concat(&mut new_nfa);
            }
            ParseElement::Group(grp) => {
                let mut new_nfa = parse(grp.clone());
                groups.push(new_nfa.clone());
                new_nfa.add_modifier(modifier);
                curr_nfa.concat(&mut new_nfa);
            }
            ParseElement::BackReference(n) => {
                let mut new_nfa = groups[(*n as usize) - 1].clone();
                new_nfa.add_modifier(modifier);
                curr_nfa.concat(&mut new_nfa);
            }
            ParseElement::Star
            | ParseElement::Plus
            | ParseElement::Question
            | ParseElement::Range(_, _) => {
                panic!("Unexpected modifier!");
            }
            _ => panic!("Unknown token {tok:?}"),
        }
    }

    while !union_stack.is_empty() {
        curr_nfa.union(&mut union_stack.pop().unwrap());
    }
    curr_nfa
}
