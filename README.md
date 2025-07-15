# regex-rs

Regex automata construction, minimization, visualization, and simulation.

## Example `ab(34)+|12(34)*`

![NFA](./doc/nfa1.png)
![Unminimized DFA](./doc/dfa_nonmin1.png)
![Minimized DFA](./doc/dfa_min1.png)

## Dependencies

- Graphviz

## Supported Syntax and Notes

The alphabet consists of all unicode scalar values.

All base regex operations (concatenation, union (`|`), groups (`(...)`), and Kleene star (`*`)) are supported.

Additionally supported:

- Wildcard `.` (WIP)
- Multiplicity metacharacters `+`, `?`
- Multiplicity ranges `{min, max}`
- Character ranges `[...]`
- Character classes `\w`, `\d`, `\s`
- Scoped backreferences (`\n` where `1 <= n <= u64::MAX`)
