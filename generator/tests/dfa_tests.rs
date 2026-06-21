use generator::core::regex::ast::RegexType;
use generator::scanner::dfa::build_dfa;
use generator::scanner::nfa::build_nfa;

#[test]
fn test_minimization() {
    let patterns = vec![(
        0,
        RegexType::Alt(
            Box::new(RegexType::Char('a' as u32)),
            Box::new(RegexType::Char('b' as u32)),
        ),
    )];
    let nfa = build_nfa(&patterns);
    let mut dfa = build_dfa(&nfa);
    let states_before = dfa.transitions.len();
    dfa.minimize();
    let states_after = dfa.transitions.len();
    assert!(states_after <= states_before);
    assert!(states_after >= 2);
}
