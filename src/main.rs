mod nfa;
mod parse;

use colored::Colorize;
use text_io::read;

use parse::{lex, parse};

fn main() {
    let toks = lex(read!("{}\n"));
    println!("toks: {:?}", toks);
    let nfa = parse(toks);
    println!("nfa: {:?}", nfa);

    print!("{}", "> ".green().bold());
    let mut input: String = read!("{}\n");
    while input != "exit" {
        let sim = nfa.simulate(input);
        match sim {
            Ok(_) => println!("{}{:?}", "Output: ".green(), sim),
            Err(_) => println!("{}{:?}", "Output: ".red(), sim),
        }
        print!("{}", "> ".green().bold());
        input = read!("{}\n");
    }
}
