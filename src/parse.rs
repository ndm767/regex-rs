use crate::nfa::Nfa;
use crate::transition_table::Transition;

#[derive(Debug, Clone)]
pub enum ParseElement {
    Literal(char), // a single character
    Wildcard,      // . matches any character

    Star,            // * matches 0 or more times
    Plus,            // + matches 1 or more times
    Question,        // ? matches 0 or 1 times
    Range(u64, u64), // a{3,5} matches aaa, aaaa, aaaaa
    OpenRange(u64),  // a{n,} matches a n or more times

    Union, // |

    Group(Vec<ParseElement>), // (...)
    Bracket(Vec<char>),       // [A-Za-z]

    BackReference(u64), //\n where n>=1, POSIX regex only mandates 1-9
}

impl ParseElement {
    fn is_modifier(&self) -> bool {
        matches!(
            self,
            Self::Star | Self::Plus | Self::Question | Self::Range(_, _) | Self::OpenRange(_)
        )
    }
}

fn get_character_class(c: char) -> Vec<char> {
    match c {
        'w' => {
            /* [A-Za-z0-9_]  */
            let mut values: Vec<char> = ('A'..='Z').collect();
            values.append(&mut ('a'..='z').collect());
            values.append(&mut ('0'..='9').collect());
            values.push('_');
            values
        }
        'd' => {
            /* [0-9] */
            ('0'..='9').collect()
        }
        's' => {
            /* [ \t] */
            vec![' ', '\t']
        }
        _ => {
            panic!("{c} is not a supported character class!")
        }
    }
}

fn get_escaped(iter: &mut impl Iterator<Item = char>) -> char {
    let next = iter.next().unwrap();

    match next {
        '.' | '*' | '+' | '?' | '{' | '}' | '|' | '^' | '$' | '(' | ')' | '[' | ']' | '-'
        | '\\' => next,
        't' => '\t',
        'x' => {
            let mut n = iter.next().unwrap().to_digit(16).unwrap();
            n *= 16;
            n += iter.next().unwrap().to_digit(16).unwrap();
            char::from_u32(n).unwrap()
        }
        'u' => {
            let mut n = 0u32;
            for _ in 0..4 {
                n *= 16;
                n += iter.next().unwrap().to_digit(16).unwrap();
            }
            char::from_u32(n).unwrap()
        }
        _ => panic!("Unknown escape character {next}!"),
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
                let mut done = false;

                while iter.peek().unwrap().is_ascii_digit() {
                    min *= 10;
                    min += iter.next().unwrap().to_digit(10).unwrap() as u64;
                }

                // consume until comma or close curly
                while !matches!(iter.peek().unwrap(), ',') && !matches!(iter.peek().unwrap(), '}') {
                    let _ = iter.next();
                }

                // exact range, i.e. a{3}
                if iter.next().unwrap() == '}' {
                    curr.push(ParseElement::Range(min, min));
                    done = true;
                }

                // consume until next digit
                while !done && !iter.peek().unwrap().is_ascii_digit() {
                    // open range, i.e. a{3,}
                    if iter.next().unwrap() == '}' {
                        curr.push(ParseElement::OpenRange(min));
                        done = true;
                    }
                }

                if !done {
                    while iter.peek().unwrap().is_ascii_digit() {
                        max *= 10;
                        max += iter.next().unwrap().to_digit(10).unwrap() as u64;
                    }

                    // consume until close curly
                    while !matches!(iter.next().unwrap(), '}') {}

                    curr.push(ParseElement::Range(min, max));
                }
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
                    match iter.next().unwrap() {
                        '\\' => match iter.peek().unwrap() {
                            'w' | 'd' | 's' => {
                                values.extend(get_character_class(iter.next().unwrap()));
                            }
                            _ => values.push(get_escaped(&mut iter)),
                        },
                        '-' => {
                            // plain hyphen is valid if it is the first or last character
                            if values.is_empty() || *iter.peek().unwrap() == ']' {
                                values.push('-');
                            } else {
                                let prev = values.pop().unwrap();
                                let end = iter.next().unwrap();
                                for c in prev..=end {
                                    values.push(c);
                                }
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
                match iter.peek().unwrap() {
                    'w' | 'd' | 's' => {
                        // character classes are treated like brackets
                        curr.push(ParseElement::Bracket(get_character_class(
                            iter.next().unwrap(),
                        )));
                    }

                    '0'..='9' => {
                        // digits
                        let mut n: u64 = iter.next().unwrap().to_digit(10).unwrap() as u64;
                        while iter.peek().unwrap().is_ascii_digit() {
                            n *= 10;
                            n += iter.next().unwrap().to_digit(10).unwrap() as u64;
                        }
                        curr.push(ParseElement::BackReference(n));
                    }

                    _ => {
                        curr.push(ParseElement::Literal(get_escaped(&mut iter)));
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
            Some(m) if m.is_modifier() => Some(tok_iter.next().unwrap().clone()),
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
            | ParseElement::Range(_, _)
            | ParseElement::OpenRange(_) => {
                panic!("Unexpected modifier!");
            }
        }
    }

    while !union_stack.is_empty() {
        curr_nfa.union(&mut union_stack.pop().unwrap());
    }
    curr_nfa
}
